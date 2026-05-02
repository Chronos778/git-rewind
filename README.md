# Rewind ⏪

An AI-powered CLI tool that instantly tells you where you left off in your Git repository. 

Run it, get briefed. No manual notes, no journals, fully automatic.

## The Problem
You work on a feature, switch to a bugfix, leave for the weekend, and come back on Monday with no idea what you were doing. You start running `git status`, `git log`, `git diff` trying to reconstruct your train of thought.

## The Solution
`rewind` analyzes your repository state (branch, recent commits, staged and unstaged changes) and feeds it to an LLM to give you a personalized, conversational briefing on what you were working on and what you left unfinished.

## Installation

```bash
cargo install --path .
```

## Configuration

`rewind` is designed strictly with free tiers in mind, along with traditional paid tiers. It automatically checks your environment variables for keys to several top providers:

To use **Groq** (insanely fast, generous free tier):
```bash
export GROQ_API_KEY="gsk_..."
```

To use **Gemini** (huge free tier, context window):
```bash
export GEMINI_API_KEY="AIza..."
```

To use **OpenAI**:
```bash
export OPENAI_API_KEY="sk-..."
```

*(Note: If multiple keys are set, it prioritizes Groq > Gemini > OpenAI to help limit accidental costs).*

### Custom Models / Local LLMs (Ollama, vLLM)
You can override the API base and model used by setting these variables (works perfectly with local servers like Ollama!):
```bash
export OPENAI_API_BASE="http://localhost:11434/v1"
export OPENAI_MODEL="llama3-8b-8192"
export OPENAI_API_KEY="ignore" # If using a local tool that ignores keys
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
