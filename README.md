# Multi-Platform Articles (MPA)

MPA 是一个将 Markdown 转换为带内置精美主题的 HTML，并可一键发布到微信公众号草稿箱（未来支持更多平台）的命令行工具。

本项目内置 48 种排版主题，且完全支持本地渲染，无需依赖外部 API。

## 特性

- **48 种内置主题**：包含科技、简约、经典等多种风格，即刻可用。
- **本地渲染**：无需调用远端服务，安全且快速。
- **一键发布**：自带图床占位替换能力，可直接发布到微信公众号草稿箱。
- **交互式配置**：提供终端 UI (TUI) 界面，配置 AppID/Secret 更直观。
- **无感安装**：支持一键安装，自动配置 PATH，解压即用。

## 快速开始

### 1. 安装 CLI (傻瓜式)

前往 [GitHub Releases](https://github.com/shijianzhong/multi-platform-articles/releases/latest) 下载对应你系统的压缩包（如 `mpa-v0.1.3-x86_64-apple-darwin.tar.gz`）。

解压并进入目录，执行一键安装：

```bash
tar -xzf mpa-v0.1.3-x86_64-apple-darwin.tar.gz
cd mpa-v0.1.3-x86_64-apple-darwin

# 自动把 mpa 安装到 ~/.local/bin 并配置 PATH
./mpa install
```

> **注意**：如果是 Windows，请在 PowerShell 中执行 `.\mpa.exe install`。

安装完成后，新开一个终端窗口（或按照提示 `source ~/.zshrc`），确保可以在任意路径执行：

```bash
mpa --version
```

### 2. 配置微信公众号

终端直接输入 `mpa` 或 `mpa tui` 进入配置界面：

```bash
mpa
```
在界面中输入你的 `AppID` 和 `Secret`，按 `s` 保存。配置会安全地存放在你的本地用户目录中。

### 3. 使用工作流

**查看所有可用主题**：
```bash
mpa themes list
```

**将 Markdown 转换为带主题的 HTML**：
```bash
mpa convert your_article.md --mode local --theme github-readme -o out.html
```

**推送到微信草稿箱**：
```bash
mpa publish wechat-draft --html out.html --title "我的第一篇测试文章" --cover path/to/cover.jpg
```

---

## 配合 Trae / ClawHub 使用

如果你使用 Trae IDE 或龙虾 (ClawHub) 助手，可以安装 `multi-platform-articles` Skill。

**安装步骤**：
1. 按照上述步骤先安装好 `mpa` CLI。
2. 通过 ClawHub 或复制本项目中的 `.trae/skills/multi-platform-articles/` 文件夹到你的项目或全局目录。
3. 直接在对话中告诉助手：“帮我用 mpa 的 github-readme 主题排版这篇文章并推送到公众号”，助手将自动调用本地 `mpa` 为你完成全套工作。
