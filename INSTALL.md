# Installation Guide

## Quick Install

### Using the install.sh script (Recommended)
```bash
# Extract the archive
tar -xzf revu-linux-amd64.tar.gz
cd revu-linux-amd64/

# Run the install script
./install.sh
```

This will:
- Install the `revu` binary to `~/.local/bin`
- Copy theme files to `~/.config/revu/themes/`
- Ensure `~/.local/bin` is in your PATH

### Manual Installation
```bash
# Copy the binary to a location in your PATH
cp revu ~/.local/bin/
chmod +x ~/.local/bin/revu

# Copy themes (optional)
mkdir -p ~/.config/revu/themes
cp themes/*.toml ~/.config/revu/themes/
```

## Authentication Setup

Revu requires a GitHub personal access token. Choose one of these methods:

### Option 1: Authinfo File (Recommended)
Create `~/.authinfo` or `~/.netrc`:
```
machine api.github.com login YOUR_USERNAME^revu password YOUR_GITHUB_TOKEN
```

Secure the file:
```bash
chmod 600 ~/.authinfo
```

### Option 2: Environment Variable
```bash
export GITHUB_TOKEN="your-personal-access-token"
```

### Option 3: Command Line
```bash
revu --token YOUR_TOKEN https://github.com/owner/repo/pull/123
```

## Usage

```bash
# Review a PR using its full URL
revu https://github.com/owner/repo/pull/123

# Review a PR using just the number (requires GITHUB_OWNER and GITHUB_REPO env vars)
revu 123
```

## Requirements

- Linux x86_64 system
- GitHub personal access token with repo access
- Terminal with 256-color support

## Troubleshooting

If `revu` is not found after installation:
```bash
# Add ~/.local/bin to your PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

For more information, visit: https://github.com/yourusername/revu