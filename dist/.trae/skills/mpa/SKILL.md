---
name: "mpa"
description: "将 Markdown 转换为 48 种内置主题 HTML，并可推送到微信公众号草稿箱。用户提到“排版/主题/发布公众号/草稿/小红书/头条”时调用。"
---

# Multi-Platform Articles (mpa)

本 skill 依赖本机已安装 `mpa` 可执行文件（方案 A：GitHub Release 预编译二进制）。

## 安装 mpa（推荐）

### macOS / Linux

```bash
export MPA_VERSION="v0.1.0"
curl -fsSL https://raw.githubusercontent.com/shijianzhong/multi-platform-articles/main/core/scripts/install.sh | sh
```

### Windows PowerShell

```powershell
$env:MPA_VERSION = "v0.1.0"
iex ((New-Object System.Net.WebClient).DownloadString("https://raw.githubusercontent.com/shijianzhong/multi-platform-articles/main/core/scripts/install.ps1"))
```

如果你想改安装目录：

```bash
export MPA_INSTALL_DIR="$HOME/.local/bin"
```

## 常用命令

### 1) 列出主题

```bash
mpa themes list
```

查看主题详情：

```bash
mpa themes show github-readme
```

### 2) Markdown 转主题 HTML

```bash
mpa convert path/to/article.md --mode local --theme github-readme -o out.html
```

### 3) 推送到微信公众号草稿箱（图文）

前置：设置环境变量（不要把密钥写进仓库）

```bash
export WECHAT_APPID="xxx"
export WECHAT_SECRET="yyy"
```

推送草稿（cover 会先上传为图片素材，返回草稿 media_id）：

```bash
mpa publish wechat-draft --html out.html --title "文章标题" --cover path/to/cover.jpg
```

可选：

```bash
--author "作者名"
--digest "摘要"
```

## 重要约束

- 公众号编辑器对 HTML/CSS 有限制：只用安全标签 + 内联 style；不要使用 <style>、脚本、外链 CSS。
- 当前 `wechat-draft` 会推送 HTML + 封面；正文图片自动上传/回填可在下一步补齐（会复用 core 的资产管线接口）。
