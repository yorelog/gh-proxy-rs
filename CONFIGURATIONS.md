# 配置示例

本文档提供了各种配置场景的示例，帮助您快速配置 gh-proxy-rs。

## 基础配置

### 默认配置（推荐）
```rust
// 不限制文件大小
const SIZE_LIMIT: u64 = 0;

// 所有列表为空，允许访问所有用户和仓库
const BANNED_LIST: &[&str] = &[];
const ALLOWED_LIST: &[&str] = &[];
const PASSBY_LIST: &[&str] = &[];
```

## 高级配置示例

### 1. 限制大文件下载
```rust
// 限制文件大小为 50MB，超过则重定向到原地址
const SIZE_LIMIT: u64 = 50 * 1024 * 1024;

// 或者禁用文件大小检查
// const SIZE_LIMIT: u64 = 0;
```

### 2. 封禁特定用户/仓库
```rust
const BANNED_LIST: &[&str] = &[
    "github.com/spam-user",           // 封禁整个用户
    "github.com/user/private-repo",   // 封禁特定仓库
    "github.com/malicious-org",       // 封禁整个组织
];
```

### 3. 仅允许特定用户/仓库
```rust
const ALLOWED_LIST: &[&str] = &[
    "github.com/trusted-user",        // 仅允许可信用户
    "github.com/company",             // 仅允许公司组织
    "github.com/user/public-project", // 仅允许特定项目
];
```

### 4. 直通（绕过代理）特定用户/仓库
```rust
const PASSBY_LIST: &[&str] = &[
    "github.com/cdn-friendly",        // 这些用户的仓库直接访问原地址
    "github.com/fast-server/repo",    // 这个仓库本身就很快，不需要代理
];
```

## 组合配置示例

### 企业环境配置
```rust
// 限制文件大小为 200MB
const SIZE_LIMIT: u64 = 200 * 1024 * 1024;

// 封禁已知的恶意用户
const BANNED_LIST: &[&str] = &[
    "github.com/malicious-user",
    "github.com/spam-org",
];

// 仅允许公司相关的仓库
const ALLOWED_LIST: &[&str] = &[
    "github.com/my-company",          // 公司组织
    "github.com/partner-company",     // 合作伙伴
    "github.com/employee1",           // 员工个人账户
    "github.com/employee2",
];

// 某些仓库直接访问，不走代理
const PASSBY_LIST: &[&str] = &[
    "github.com/my-company/public-docs", // 公开文档仓库
];
```

### 开源项目镜像配置
```rust
// 不限制文件大小
const SIZE_LIMIT: u64 = 0;

// 封禁已知问题仓库
const BANNED_LIST: &[&str] = &[
    "github.com/copyright-violator",
    "github.com/malware-repo",
];

// 不限制访问用户（空白名单）
const ALLOWED_LIST: &[&str] = &[];

// 对于已经有好的 CDN 的项目直接跳转
const PASSBY_LIST: &[&str] = &[
    "github.com/microsoft",           // 微软的仓库通常有好的 CDN
    "github.com/google",              // Google 的仓库
    "github.com/facebook",            // Meta 的仓库
];
```

### 个人使用配置
```rust
// 限制大文件为 500MB
const SIZE_LIMIT: u64 = 500 * 1024 * 1024;

// 不封禁任何用户
const BANNED_LIST: &[&str] = &[];

// 不限制访问
const ALLOWED_LIST: &[&str] = &[];

// 个人常用且速度快的仓库直接访问
const PASSBY_LIST: &[&str] = &[
    "github.com/myself",              // 自己的仓库
    "github.com/friend/fast-repo",    // 朋友的快速仓库
];
```

## 配置优先级说明

配置的检查顺序和逻辑：

1. **直通检查** (`PASSBY_LIST`)
   - 如果 URL 匹配直通列表，直接重定向到原地址
   - 跳过所有后续检查

2. **封禁检查** (`BANNED_LIST`)  
   - 如果 URL 匹配封禁列表，返回 403 错误
   - 优先级高于白名单

3. **白名单检查** (`ALLOWED_LIST`)
   - 如果白名单非空且 URL 不匹配，返回 403 错误
   - 如果白名单为空，跳过此检查

4. **文件大小检查**
   - 对符合条件的文件类型进行大小检查
   - 超过限制则重定向到原地址

5. **正常代理**
   - 通过所有检查后进行正常的代理转发

## 测试配置

配置完成后，可以通过以下方式测试：

```bash
# 测试封禁用户（应该返回 403）
curl -I https://your-worker.dev/https://github.com/banned-user/repo/archive/main.zip

# 测试直通用户（应该返回 302 重定向到原地址）
curl -I https://your-worker.dev/https://github.com/passby-user/repo/archive/main.zip

# 测试大文件（应该返回 302 重定向到原地址，如果文件超过限制）
curl -I https://your-worker.dev/https://github.com/user/repo/releases/download/v1.0/large-file.zip
```

## 注意事项

1. **匹配规则**: 所有匹配都使用 `starts_with` 逻辑，所以 `"github.com/user"` 会匹配 `"github.com/user/any-repo"`

2. **性能考虑**: 文件大小检查需要额外的 HEAD 请求，会稍微增加延迟

3. **正则表达式**: 配置中的字符串是精确匹配，不支持正则表达式

4. **大小写敏感**: 所有匹配都是大小写敏感的

5. **更新配置**: 修改配置后需要重新部署 Worker 才能生效