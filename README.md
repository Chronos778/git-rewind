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

Set your OpenAI API key (or any OpenAI-compatible API):

```bash
export OPENAI_API_KEY="your-api-key"
```

You can also use alternative models/APIs by setting:
```bash
export OPENAI_API_BASE="https://api.openai.com/v1"
export OPENAI_MODEL="gpt-4o"
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
