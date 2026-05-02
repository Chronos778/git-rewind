# Rewind

An AI-powered CLI tool that instantly tells you where you left off in your Git repository. 

Run it, get briefed. No manual notes, no journals, fully automatic.

## The Problem
You work on a feature, switch to a bugfix, leave for the weekend, and come back on Monday with no idea what you were doing. You start running `git status`, `git log`, `git diff` trying to reconstruct your train of thought.

## The Solution
`rewind` analyzes your repository state (branch, recent commits, staged and unstaged changes) and feeds it to an LLM to give you a personalized, conversational briefing on what you were working on and what you left unfinished.

## Installation

### Option 1: Automatic Install Scripts (Recommended)
You do not need Rust or developer tools installed. These scripts will download the latest binary, place it in an appropriate folder (`~/.local/bin` for Unix, `%USERPROFILE%\.rewindin` for Windows), and automatically add it to your system's PATH.

**Windows (PowerShell):**
```powershell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/Chronos778/git-rewind/main/install.ps1" -OutFile "$env:TEMP
ewind_install.ps1"
powershell -ExecutionPolicy Bypass -File "$env:TEMP
ewind_install.ps1"
```
*(If Windows Defender says it's "not safe to run" after installation, it's just because the binary is unsigned. Click "More info" -> "Run anyway", or use Option 2 below if you prefer to compile it yourself.)*

**Linux / macOS (Bash):**
```bash
curl -fsSL https://raw.githubusercontent.com/Chronos778/git-rewind/main/install.sh | bash
```

### Option 2: Using Cargo (For Rust Developers)
If you already have the Rust toolchain installed, you can compile and install it directly from this repository. This bypasses the unsigned binary warning on Windows entirely since it compiles locally!
```bash
cargo install --git https://github.com/Chronos778/git-rewind.git
```

### Option 3: Manual Pre-compiled Binaries
1. Go to the [Releases](https://github.com/Chronos778/git-rewind/releases) page of this repository.
2. Download the archive for your operating system (`.zip` for Windows, `.tar.gz` for macOS/Linux).
3. Extract the `rewind` executable and add it to your PATH manually.

## Configuration & Commands

By default, the first time you run `rewind`, it will launch an interactive setup prompting you to paste an API key. 
You can use the new `config` command to manually add, view, or remove multiple API keys:

```bash
# View your saved keys (redacted)
rewind config show

# Add or change a specific provider's key
rewind config set groq gsk_123456789...
rewind config set gemini AIzaSyB...
rewind config set openai sk-proj-...

# Delete a key
rewind config clear openai
```

Alternatively, `rewind` checks your environment variables for keys to several top providers:

To use **Groq** (insanely fast, generous free tier):
```bash
# Linux/macOS:
export GROQ_API_KEY="gsk_..."

# Windows PowerShell:
$env:GROQ_API_KEY="gsk_..."
```

To use **Gemini** (huge free tier, context window):
```bash
# Linux/macOS:
export GEMINI_API_KEY="AIza..."

# Windows PowerShell:
$env:GEMINI_API_KEY="AIza..."
```

To use **OpenAI**:
```bash
# Linux/macOS:
export OPENAI_API_KEY="sk-..."

# Windows PowerShell:
$env:OPENAI_API_KEY="sk-..."
```

*(Note: If multiple keys are set, it prioritizes Groq > Gemini > OpenAI to help limit accidental costs).*

### Custom Models / Local LLMs (Ollama, vLLM)
You can override the API base and model used by setting these variables (works perfectly with local servers like Ollama!):
```bash
# Linux/macOS:
export OPENAI_API_BASE="http://localhost:11434/v1"
export OPENAI_MODEL="llama3-8b-8192"
export OPENAI_API_KEY="ignore" # If using a local tool that ignores keys

# Windows PowerShell:
$env:OPENAI_API_BASE="http://localhost:11434/v1"
$env:OPENAI_MODEL="llama3-8b-8192"
$env:OPENAI_API_KEY="ignore"
```

## Usage

Simply run:
```bash
rewind
```

If you want to see what data `rewind` sends to the AI, use:
```bash
rewind --dry-run
```
