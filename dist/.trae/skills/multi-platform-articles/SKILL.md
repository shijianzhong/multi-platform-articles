---
name: "multi-platform-articles"
description: "将 Markdown 转换为 48 种内置主题 HTML，并可推送到微信公众号草稿箱。用户提到“排版/主题/发布公众号/草稿/小红书/头条”时调用。"
---

# Multi-Platform Articles

本 Skill 只负责知识与工作流编排，不负责安装/下载任何可执行文件。使用前请先在本机安装 `mpa` CLI（并确保终端里能直接运行 `mpa`）。

## 最省事安装（推荐）

如果你下载的是 GitHub Release 的压缩包，解压后在解压目录执行：

```bash
./mpa install
```

它会把 `mpa` 安装到 `~/.local/bin`（并尝试自动写入 `~/.zshrc`/`~/.bashrc`），同时把本 Skill 安装到 `~/.trae/skills/multi-platform-articles/`（如果压缩包内包含该目录）。

## 前置检查

```bash
mpa --help
```

## 配置（推荐用 TUI）

```bash
mpa
```

或：

```bash
mpa tui
```

在 TUI 里填写/修改 `WECHAT_APPID`、`WECHAT_SECRET` 并按 `s` 保存。

## 常用工作流

### 1) 列出主题

```bash
mpa themes list
```

查看主题详情：

```bash
mpa themes show github-readme
```

### 2) Markdown 转主题 HTML（本地渲染）

```bash
mpa convert path/to/article.md --mode local --theme github-readme -o out.html
```

### 3) 推送微信公众号草稿箱（图文）

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
- 环境变量优先于本机配置文件：若设置了 `WECHAT_APPID/WECHAT_SECRET`，将覆盖 TUI 保存的值。
