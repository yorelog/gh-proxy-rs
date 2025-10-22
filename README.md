# gh-proxy-rs

GitHub 文件加速项目的 Rust 实现版本，基于 Cloudflare Workers 和 [workers-rs](https://github.com/cloudflare/workers-rs)。

这是 [hunshcn/gh-proxy](https://github.com/hunshcn/gh-proxy) 项目的 Rust 移植版本，感谢原作者提供的优秀实现思路。

## ✨ 特性

- 🚀 **高性能**: 使用 Rust 编写，WebAssembly 运行，性能优异
- 🌐 **完整支持**: 支持 GitHub release、archive、raw 文件、gist 等
- 🔄 **Git 操作**: 支持 `git clone` 等 Git 协议操作
- 🔒 **私有仓库**: 支持通过 token 访问私有仓库
- 🎯 **灵活配置**: 支持白名单、jsDelivr CDN 切换等配置
- 🌍 **CORS 支持**: 完整的跨域资源共享支持
- 📏 **文件大小限制**: 超过设定大小自动返回原地址，避免大文件占用资源
- 🛡️ **访问控制**: 支持用户/仓库级别的封禁、白名单和直通机制

## 📖 使用说明

### 方法一：直接在 URL 前添加代理地址

将需要加速的 GitHub 链接前面加上部署的 Worker 地址即可。

示例（假设部署地址为 `https://gh-proxy.example.com`）:

```
https://gh-proxy.example.com/https://github.com/owner/repo/releases/download/v1.0.0/file.zip
```

### 方法二：通过查询参数

```
https://gh-proxy.example.com/?q=https://github.com/owner/repo/archive/main.zip
```

### 支持的 URL 类型

以下都是合法输入（仅示例，文件不存在）：

- **分支源码**: `https://github.com/hunshcn/project/archive/master.zip`
- **Release 源码**: `https://github.com/hunshcn/project/archive/v0.1.0.tar.gz`
- **Release 文件**: `https://github.com/hunshcn/project/releases/download/v0.1.0/example.zip`
- **分支文件**: `https://github.com/hunshcn/project/blob/master/filename`
- **Commit 文件**: `https://github.com/hunshcn/project/blob/1111111111111111111111111111/filename`
- **Gist**: `https://gist.githubusercontent.com/user/id/raw/file.py`
- **Raw 文件**: `https://raw.githubusercontent.com/owner/repo/main/file.txt`

### 访问私有仓库

通过在 URL 中添加 token 的方式访问私有仓库：

```bash
git clone https://user:TOKEN@gh-proxy.example.com/https://github.com/owner/private-repo.git
```

## 🚀 部署指南

### Cloudflare Workers 部署

1. **注册 Cloudflare 账号**
   
   访问 https://workers.cloudflare.com 注册并登录

2. **克隆本项目**

   ```bash
   git clone https://github.com/yorelog/gh-proxy-rs.git
   cd gh-proxy-rs
   ```

3. **安装依赖**

   ```bash
   npm install
   ```

4. **配置 Wrangler**

   编辑 `wrangler.toml` 文件，修改 `name` 字段为你想要的 Worker 名称

5. **部署**

   ```bash
   npx wrangler deploy
   ```

部署成功后，Wrangler 会显示你的 Worker 地址。

### 本地开发

```bash
# 安装 Rust（如果还没有安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 本地运行
npx wrangler dev
```

## ⚙️ 配置说明

编辑 `src/lib.rs` 中的常量来自定义行为：

```rust
// 静态资源 URL（首页 HTML）
const ASSET_URL: &str = "https://yorelog.github.io/gh-proxy/";

// URL 前缀，如果使用自定义路由如 example.com/gh/*，改为 "/gh/"
const PREFIX: &str = "/";

// 是否启用 jsDelivr CDN（对于 blob 文件）
const JSDELIVR_ENABLED: bool = false;

// 白名单，只有包含这些字符串的路径才会被代理
// 空数组表示允许所有路径
const WHITE_LIST: &[&str] = &[];
// 示例：只允许特定用户的仓库
// const WHITE_LIST: &[&str] = &["/username/"];

// 文件大小限制（字节）- 设置为 0 表示不限制文件大小
// 示例：限制为 100MB 可设置为 100 * 1024 * 1024
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
```

### 🔧 高级配置说明

#### 文件大小限制
- `SIZE_LIMIT`: 设置文件大小限制（以字节为单位）
- 当请求的文件大小超过此限制时，会自动重定向到原始 GitHub 地址
- 设置为 `0` 表示不限制文件大小（默认值）
- 示例：限制为 100MB 可设置为 `100 * 1024 * 1024`

#### 用户/仓库访问控制
支持三种类型的访问控制列表：

1. **封禁列表** (`BANNED_LIST`)
   - 在此列表中的用户/仓库将被拒绝访问
   - 支持用户级别封禁：`"github.com/username"`
   - 支持仓库级别封禁：`"github.com/username/repository"`

2. **白名单** (`ALLOWED_LIST`) 
   - 如果此列表非空，只有列表中的用户/仓库被允许访问
   - 列表为空时表示允许所有用户/仓库（除封禁列表外）
   - 支持用户级别许可：`"github.com/username"`
   - 支持仓库级别许可：`"github.com/username/repository"`

3. **直通列表** (`PASSBY_LIST`)
   - 在此列表中的用户/仓库将绕过代理，直接重定向到原始地址
   - 适用于不需要代理加速的可信用户/仓库
   - 支持用户级别直通：`"github.com/username"`
   - 支持仓库级别直通：`"github.com/username/repository"`

#### 优先级顺序
访问控制的检查顺序如下：
1. 直通列表 → 如果匹配，直接重定向到原地址
2. 封禁列表 → 如果匹配，返回 403 错误
3. 白名单 → 如果白名单非空且不匹配，返回 403 错误
4. 其他情况 → 允许访问并进行代理

## 💰 Cloudflare Workers 计费

- **免费版**: 每天 10 万次请求，每分钟 1000 次请求限制
- **付费版**: $5/月，每月 1000 万次请求（超出部分 $0.5/百万次）

查看使用情况：进入 Cloudflare Workers Dashboard 的 `Overview` 页面

## 🆚 与原项目的差异

### 优势

- ✅ **更高性能**: Rust + WebAssembly，执行效率更高
- ✅ **类型安全**: 编译期类型检查，运行时更稳定
- ✅ **更小体积**: 编译后的 Wasm 体积更小
- ✅ **现代化**: 使用最新的 workers-rs SDK

### 功能差异

与原 Python 版本的功能对比：
- ✅ 文件大小限制（超过设定返回原地址）
- ✅ 特定 user/repo 的封禁/白名单/passby 机制
- ✅ 完整的 GitHub 代理功能
- ✅ 高性能 Rust + WebAssembly 实现

本项目现已实现原 Python 版本的所有核心功能，并在性能上有显著提升。

## 📝 更新日志

- **2025.10.22**: 初始版本，基于 workers-rs 实现完整的 GitHub 代理功能

## 🙏 致谢

本项目基于 [hunshcn/gh-proxy](https://github.com/hunshcn/gh-proxy) 项目，感谢原作者 [@hunshcn](https://github.com/hunshcn) 的创意和实现。

同时感谢：
- [cloudflare/workers-rs](https://github.com/cloudflare/workers-rs) - Rust SDK for Cloudflare Workers
- [EtherDream/jsproxy](https://github.com/EtherDream/jsproxy/) - 参考项目

## 📄 开源协议

本项目采用 MIT 协议开源，详见 [LICENSE](LICENSE) 文件。

## 🔗 相关链接

- [原项目 gh-proxy](https://github.com/hunshcn/gh-proxy)
- [workers-rs](https://github.com/cloudflare/workers-rs)
- [Cloudflare Workers 文档](https://developers.cloudflare.com/workers/)
