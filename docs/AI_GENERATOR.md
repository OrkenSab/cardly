# Cardly AI Card Generator Guide

> **Cardly AI Card Generator** turns lecture transcripts, textbook excerpts, research articles, and photos/diagrams into clean, atomic spaced repetition flashcards for the **FSRS-6** scheduler (same as upstream Anki 26.09.4).

---

## 🌟 Key Features

- **Multimodal Source Support**:
  - **Text & Study Notes**: Paste raw notes, bullet points, Markdown, or book chapters.
  - **Photo & Document OCR**: Upload smartphone photos of textbook pages, handwritten notes, whiteboards, or diagrams.
- **Card Generation Modes**:
  - **Basic (Question & Answer)**: Atomic front/back cards targeting single facts or concepts.
  - **Cloze Deletion (`{{c1::...}}`)**: Sentence-based contextual retrieval deletions.
- **Provider Flexibility**:
  - **Google Gemini** (Recommended for speed and multimodal image OCR).
  - **OpenAI & OpenRouter** (`gpt-4o-mini`, `deepseek-chat`, etc.).
  - **Local Ollama** (100% offline, zero data leaves your local machine).
- **Interactive Review Step**:
  - Preview all generated cards before adding them to your collection.
  - Edit prompts, answers, and tags in real time.
  - Selectively check/uncheck cards or delete unwanted items.
  - One-click batch addition into your chosen deck.

---

## 🚀 Supported AI Providers

### 1. Google Gemini (Recommended)
Google Gemini provides fast inference, generous free tier quotas, and multimodal image understanding.

- **Recommended Models**: `gemini-1.5-flash` (fast, high throughput), `gemini-1.5-pro` (complex reasoning).
- **Setup**:
  1. Obtain a free API key at [Google AI Studio](https://aistudio.google.com/).
  2. In Cardly, open **Tools &rarr; AI Card Generator...** (or click **✨ AI** in the top toolbar, shortcut `Ctrl+Shift+G`).
  3. Expand **⚙️ AI Provider**, select **Google Gemini**, and paste your API key.
  4. Your API key is stored securely in your browser's `localStorage` and is never logged on the server.

### 2. OpenAI & OpenRouter
Supports standard OpenAI models as well as community providers through OpenRouter.

- **Recommended Models**: `gpt-4o-mini`, `gpt-4o`, `deepseek/deepseek-chat` (via OpenRouter).
- **Setup**:
  1. Obtain an API key from [OpenAI](https://platform.openai.com/) or [OpenRouter](https://openrouter.ai/).
  2. Select **OpenAI / Ollama** in the AI Provider settings.
  3. Enter your API key and chosen model name (e.g. `gpt-4o-mini`).
  4. If using OpenRouter, set the Custom Base URL to: `https://openrouter.ai/api/v1`.

### 3. Local Ollama (100% Offline & Private)
For medical, legal, or sensitive proprietary study material where no data may leave your computer, you can run open-weights models locally.

- **Recommended Models**: `llama3.2`, `mistral`, `qwen2.5`, or `llava` (for images).
- **Setup**:
  1. Install [Ollama](https://ollama.com/) on your host machine:
     ```bash
     ollama run llama3.2
     ```
  2. If Cardly runs in Docker, configure Ollama to listen on all interfaces:
     ```bash
     OLLAMA_HOST="0.0.0.0:11434" OLLAMA_ORIGINS="*" ollama serve
     ```
  3. In Cardly's AI Provider settings:
     - Select **OpenAI / Ollama**.
     - API Key: enter any placeholder (e.g. `ollama`).
     - Model Name: `llama3.2`.
     - Custom Base URL: `http://localhost:11434/v1` (or `http://host.docker.internal:11434/v1` if Cardly is running inside a Docker container).

---

## 📖 Step-by-Step Usage Guide

### Step 1: Open the Generator
- Click the **✨ AI** button in the top toolbar, or
- Select **Tools &rarr; AI Card Generator...** from the application menu, or
- Press the keyboard shortcut: **`Ctrl+Shift+G`** (or `Cmd+Shift+G` on macOS).

### Step 2: Provide Source Material
- **Text & Notes tab**: Paste notes or article text.
- **Photo / Image tab**: Drag-and-drop a photo of a textbook, whiteboard, or diagram.

### Step 3: Configure Card Output
- **Target Deck**: Choose which deck will receive the new cards.
- **Card Type**: Select **Basic (Q&A)** or **Cloze Deletion**.
- **Card Count**: Adjust the slider (3 to 20 cards per batch).
- **Language**: Choose *Auto* (detects source text) or force *English*, *Russian*, or *Kazakh*.

### Step 4: Generate & Review
- Click **✨ Generate Flashcards**.
- The AI creates atomic cards designed for spaced repetition.
- In the **Review & Edit** screen:
  - Edit Front / Back fields or Cloze text to refine wording.
  - Add or remove tags.
  - Uncheck cards you do not wish to study.
  - Delete individual cards with the **✕** button.

### Step 5: Add to Collection
- Click **Add N Cards to Deck**.
- The cards are created in your collection with full undo support.
- Your deck view immediately updates with the new card counts.

---

## 🧠 Spaced Repetition Best Practices

The Cardly AI generator follows proven principles for long-term memory retention:

1. **The Minimum Information Principle (Atomicity)**:
   Each card tests exactly one fact, relationship, or concept. Complex questions with multi-point answers are automatically broken into separate atomic cards.
2. **Context-Rich Cloze Deletions**:
   Cloze cards include enough surrounding context to prompt recall without creating ambiguous blanks.
3. **FSRS Scheduling Harmony**:
   Clean atomic cards lead to consistent review intervals, preventing ease hell and minimizing review fatigue in the FSRS algorithm.

---

## 🔒 Security & Privacy

- **Client-Side Secret Storage**: All API keys are saved exclusively in the user's browser `localStorage`.
- **Session Protection**: All backend generation requests require valid `X-Cardly-Api-Token` headers, preventing unauthorized access from untrusted card templates.
- **Offline Option**: Users requiring absolute confidentiality can use local Ollama with zero external network connectivity.
