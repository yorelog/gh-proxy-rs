use regex::Regex;
use worker::*;

// 配置常量
const ASSET_URL: &str = "https://yorelog.github.io/gh-proxy/";
const PREFIX: &str = "/";
const JSDELIVR_ENABLED: bool = false;

// 白名单，路径里面有包含字符的才会通过
const WHITE_LIST: &[&str] = &[];

// 文件大小限制（字节）- 设置为 0 表示不限制文件大小
const SIZE_LIMIT: u64 = 0;

// 用户/仓库 封禁列表 - 这些用户/仓库将被拒绝访问
const BANNED_LIST: &[&str] = &[
    // 示例: "github.com/user1", "github.com/user2/repo"
];

// 用户/仓库 白名单 - 如果非空，只有这些用户/仓库被允许访问
const ALLOWED_LIST: &[&str] = &[
    // 示例: "github.com/user1", "github.com/user2/repo"
];

// 用户/仓库 直通列表 - 这些用户/仓库将直接返回原地址（绕过代理）
const PASSBY_LIST: &[&str] = &[
    // 示例: "github.com/user1", "github.com/user2/repo"
];

// 为了避免无限递归，需要使用 Box::pin
type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a>>;

fn create_cors_headers() -> Headers {
    let headers = Headers::new();
    headers.set("access-control-allow-origin", "*").ok();
    headers
        .set("access-control-allow-methods", "GET,POST,PUT,PATCH,TRACE,DELETE,HEAD,OPTIONS")
        .ok();
    headers.set("access-control-max-age", "1728000").ok();
    headers
}

fn create_response(body: &str, status: u16) -> Result<Response> {
    let headers = Headers::new();
    headers.set("access-control-allow-origin", "*").ok();
    let mut response = Response::ok(body)?;
    *response.headers_mut() = headers;
    Ok(response.with_status(status))
}

fn check_whitelist(url_str: &str) -> bool {
    if WHITE_LIST.is_empty() {
        return true;
    }
    
    for item in WHITE_LIST {
        if url_str.contains(item) {
            return true;
        }
    }
    false
}

/// 从 URL 中提取用户/仓库路径
fn extract_user_repo(url_str: &str) -> Option<String> {
    let url_clean = url_str
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    
    // 处理 github.com 的 URL
    if let Some(github_path) = url_clean.strip_prefix("github.com/") {
        let parts: Vec<&str> = github_path.split('/').collect();
        if parts.len() >= 2 {
            return Some(format!("github.com/{}/{}", parts[0], parts[1]));
        } else if parts.len() >= 1 && !parts[0].is_empty() {
            return Some(format!("github.com/{}", parts[0]));
        }
    }
    
    // 处理 raw.githubusercontent.com 的 URL
    if let Some(raw_path) = url_clean.strip_prefix("raw.githubusercontent.com/") {
        let parts: Vec<&str> = raw_path.split('/').collect();
        if parts.len() >= 2 {
            return Some(format!("github.com/{}/{}", parts[0], parts[1]));
        }
    }
    
    // 处理 gist 的 URL
    if url_clean.starts_with("gist.githubusercontent.com/") || url_clean.starts_with("gist.github.com/") {
        let prefix = if url_clean.starts_with("gist.githubusercontent.com/") {
            "gist.githubusercontent.com/"
        } else {
            "gist.github.com/"
        };
        
        if let Some(gist_path) = url_clean.strip_prefix(prefix) {
            let parts: Vec<&str> = gist_path.split('/').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                return Some(format!("github.com/{}", parts[0]));
            }
        }
    }
    
    None
}

/// 检查用户/仓库访问权限
/// 返回值：(允许访问, 是否需要直通)
fn check_user_repo_access(url_str: &str) -> (bool, bool) {
    let user_repo = match extract_user_repo(url_str) {
        Some(ur) => ur,
        None => return (true, false), // 无法提取用户/仓库信息，默认允许
    };
    
    // 检查直通列表
    for passby_item in PASSBY_LIST {
        if user_repo.starts_with(passby_item) {
            return (true, true); // 允许访问，需要直通
        }
    }
    
    // 检查封禁列表
    for banned_item in BANNED_LIST {
        if user_repo.starts_with(banned_item) {
            return (false, false); // 拒绝访问
        }
    }
    
    // 检查白名单（如果白名单非空）
    if !ALLOWED_LIST.is_empty() {
        for allowed_item in ALLOWED_LIST {
            if user_repo.starts_with(allowed_item) {
                return (true, false); // 允许访问，不需要直通
            }
        }
        return (false, false); // 白名单非空但不在白名单中，拒绝访问
    }
    
    (true, false) // 默认允许访问，不需要直通
}

fn check_url(url_str: &str) -> bool {
    // GitHub releases/archive
    let exp1 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/(?:releases|archive)/.*$").unwrap();
    // GitHub blob/raw
    let exp2 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/(?:blob|raw)/.*$").unwrap();
    // GitHub git operations
    let exp3 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/(?:info|git-).*$").unwrap();
    // raw.githubusercontent.com
    let exp4 = Regex::new(r"^(?:https?://)?raw\.(?:githubusercontent|github)\.com/.+?/.+?/.+?/.+$").unwrap();
    // gist
    let exp5 = Regex::new(r"^(?:https?://)?gist\.(?:githubusercontent|github)\.com/.+?/.+?/.+$").unwrap();
    // GitHub tags
    let exp6 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/tags.*$").unwrap();

    exp1.is_match(url_str)
        || exp2.is_match(url_str)
        || exp3.is_match(url_str)
        || exp4.is_match(url_str)
        || exp5.is_match(url_str)
        || exp6.is_match(url_str)
}

fn get_url_type(url_str: &str) -> UrlType {
    let exp1 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/(?:releases|archive)/.*$").unwrap();
    let exp2 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/(?:blob|raw)/.*$").unwrap();
    let exp3 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/(?:info|git-).*$").unwrap();
    let exp4 = Regex::new(r"^(?:https?://)?raw\.(?:githubusercontent|github)\.com/.+?/.+?/.+?/.+$").unwrap();
    let exp5 = Regex::new(r"^(?:https?://)?gist\.(?:githubusercontent|github)\.com/.+?/.+?/.+$").unwrap();
    let exp6 = Regex::new(r"^(?:https?://)?github\.com/.+?/.+?/tags.*$").unwrap();

    if exp1.is_match(url_str) {
        UrlType::Release
    } else if exp2.is_match(url_str) {
        UrlType::Blob
    } else if exp3.is_match(url_str) {
        UrlType::Git
    } else if exp4.is_match(url_str) {
        UrlType::Raw
    } else if exp5.is_match(url_str) {
        UrlType::Gist
    } else if exp6.is_match(url_str) {
        UrlType::Tags
    } else {
        UrlType::Asset
    }
}

enum UrlType {
    Release,
    Blob,
    Git,
    Raw,
    Gist,
    Tags,
    Asset,
}

async fn handle_preflight() -> Result<Response> {
    let headers = create_cors_headers();
    Ok(Response::empty()?.with_headers(headers).with_status(204))
}

/// 检查文件大小，如果超过限制则返回原地址的重定向响应
async fn check_file_size_and_redirect(url: &str) -> Result<Option<Response>> {
    if SIZE_LIMIT == 0 {
        return Ok(None); // 如果大小限制为0，表示不启用此功能
    }
    
    let target_url = if url.starts_with("http") {
        url.to_string()
    } else {
        format!("https://{}", url)
    };
    
    // 发送 HEAD 请求检查文件大小
    let mut init = RequestInit::new();
    init.with_method(Method::Head);
    
    let response = match Fetch::Request(Request::new_with_init(&target_url, &init)?).send().await {
        Ok(resp) => resp,
        Err(_) => return Ok(None), // 无法获取文件信息，继续代理
    };
    
    // 检查 Content-Length 头
    if let Ok(Some(content_length_str)) = response.headers().get("content-length") {
        if let Ok(content_length) = content_length_str.parse::<u64>() {
            if content_length > SIZE_LIMIT {
                // 文件过大，返回重定向到原地址
                let parsed_url = Url::parse(&target_url).map_err(|e| Error::RustError(e.to_string()))?;
                return Ok(Some(Response::redirect(parsed_url)?));
            }
        }
    }
    
    Ok(None) // 文件大小在限制内，继续代理
}

fn proxy_request<'a>(url: Url, req: Request) -> BoxFuture<'a, Result<Response>> {
    Box::pin(async move {
        proxy_request_impl(url, req).await
    })
}

async fn proxy_request_impl(url: Url, mut req: Request) -> Result<Response> {
    // 创建新的请求头
    let headers = req.headers().clone();
    
    // 发送代理请求
    let mut init = RequestInit::new();
    init.with_method(req.method());
    init.with_headers(headers.clone());
    
    if req.method() != Method::Get && req.method() != Method::Head {
        if let Ok(body) = req.bytes().await {
            init.with_body(Some(body.into()));
        }
    }

    let mut response = Fetch::Request(Request::new_with_init(url.as_str(), &init)?).send().await?;

    // 处理响应头
    let res_headers = response.headers().clone();
    
    // 处理重定向
    if let Some(location) = res_headers.get("location")? {
        if check_url(&location) {
            let new_location = format!("{}{}", PREFIX, location);
            res_headers.set("location", &new_location)?;
        } else {
            // 跟随重定向
            let new_url = Url::parse(&location).map_err(|e| Error::RustError(e.to_string()))?;
            return proxy_request(new_url, req).await;
        }
    }

    res_headers.set("access-control-expose-headers", "*")?;
    res_headers.set("access-control-allow-origin", "*")?;
    res_headers.delete("content-security-policy")?;
    res_headers.delete("content-security-policy-report-only")?;
    res_headers.delete("clear-site-data")?;

    let status = response.status_code();
    let body = response.bytes().await?;

    Ok(Response::from_bytes(body)?
        .with_headers(res_headers)
        .with_status(status))
}

#[event(fetch)]
async fn main(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    // 处理 CORS 预检请求
    if req.method() == Method::Options {
        if req.headers().get("access-control-request-headers")?.is_some() {
            return handle_preflight().await;
        }
    }

    let url = req.url()?;
    
    // 检查是否有查询参数 q (用于重定向)
    if let Some(query_path) = url.query_pairs().find(|(k, _)| k == "q") {
        let redirect_url = format!("https://{}{}{}", url.host_str().unwrap_or(""), PREFIX, query_path.1);
        return Response::redirect(Url::parse(&redirect_url)?);
    }

    // 获取路径并处理
    let origin_len = url.origin().ascii_serialization().len();
    let prefix_len = PREFIX.len();
    let full_path = url.as_str();
    
    if full_path.len() <= origin_len + prefix_len {
        return Fetch::Url(Url::parse(&format!("{}{}", ASSET_URL, ""))?).send().await;
    }

    let mut path = full_path[origin_len + prefix_len..].to_string();
    
    // 处理协议前缀
    if path.starts_with("http://") || path.starts_with("https://") {
        // 已经包含协议
    } else if path.starts_with("http:/") {
        path = format!("https://{}", path[6..].trim_start_matches('/'));
    } else if path.starts_with("https:/") {
        path = format!("https://{}", path[7..].trim_start_matches('/'));
    }

    // 检查用户/仓库访问权限
    let (access_allowed, need_passby) = check_user_repo_access(&path);
    if !access_allowed {
        return create_response("User/repository access denied", 403);
    }
    
    // 如果需要直通，直接重定向到原地址
    if need_passby {
        let target_url = if path.starts_with("http") {
            path
        } else {
            format!("https://{}", path)
        };
        let parsed_url = Url::parse(&target_url).map_err(|e| Error::RustError(e.to_string()))?;
        return Response::redirect(parsed_url);
    }

    let url_type = get_url_type(&path);

    match url_type {
        UrlType::Release | UrlType::Git | UrlType::Gist | UrlType::Tags => {
            // 检查白名单
            if !check_whitelist(&path) {
                return create_response("blocked", 403);
            }
            
            let target_url = if path.starts_with("http") {
                path
            } else {
                format!("https://{}", path)
            };
            
            // 对于 Release 类型的文件，检查文件大小
            if matches!(url_type, UrlType::Release) {
                if let Some(redirect_response) = check_file_size_and_redirect(&target_url).await? {
                    return Ok(redirect_response);
                }
            }
            
            let parsed_url = Url::parse(&target_url).map_err(|e| Error::RustError(e.to_string()))?;
            proxy_request(parsed_url, req).await
        }
        UrlType::Blob => {
            if JSDELIVR_ENABLED {
                // 使用 jsDelivr CDN
                let new_url = path
                    .replace("/blob/", "@")
                    .replace("github.com", "cdn.jsdelivr.net/gh");
                let new_url = if new_url.starts_with("http") {
                    new_url
                } else {
                    format!("https://{}", new_url.trim_start_matches('/'))
                };
                Response::redirect(Url::parse(&new_url)?)
            } else {
                // 转换为 raw 并代理
                let raw_path = path.replace("/blob/", "/raw/");
                if !check_whitelist(&raw_path) {
                    return create_response("blocked", 403);
                }
                
                let target_url = if raw_path.starts_with("http") {
                    raw_path
                } else {
                    format!("https://{}", raw_path)
                };
                
                // 检查文件大小
                if let Some(redirect_response) = check_file_size_and_redirect(&target_url).await? {
                    return Ok(redirect_response);
                }
                
                let parsed_url = Url::parse(&target_url).map_err(|e| Error::RustError(e.to_string()))?;
                proxy_request(parsed_url, req).await
            }
        }
        UrlType::Raw => {
            if JSDELIVR_ENABLED {
                // 使用 jsDelivr CDN
                let re = Regex::new(r"(?<=com/.+?/.+?)/(.+?)/").unwrap();
                let new_url = re.replace(&path, "@$1/");
                let new_url = new_url
                    .replace("raw.githubusercontent.com", "cdn.jsdelivr.net/gh")
                    .replace("raw.github.com", "cdn.jsdelivr.net/gh");
                let new_url = if new_url.starts_with("http") {
                    new_url.to_string()
                } else {
                    format!("https://{}", new_url.trim_start_matches('/'))
                };
                Response::redirect(Url::parse(&new_url)?)
            } else {
                // 直接代理
                if !check_whitelist(&path) {
                    return create_response("blocked", 403);
                }
                
                let target_url = if path.starts_with("http") {
                    path
                } else {
                    format!("https://{}", path)
                };
                
                // 检查文件大小
                if let Some(redirect_response) = check_file_size_and_redirect(&target_url).await? {
                    return Ok(redirect_response);
                }
                
                let parsed_url = Url::parse(&target_url).map_err(|e| Error::RustError(e.to_string()))?;
                proxy_request(parsed_url, req).await
            }
        }
        UrlType::Asset => {
            // 返回静态资源
            Fetch::Url(Url::parse(&format!("{}{}", ASSET_URL, path))?).send().await
        }
    }
}
