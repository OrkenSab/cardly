<script lang="ts">
  import { continueAsGuest, login, register } from "../lib/api";
  import { store } from "../lib/store.svelte";

  let mode = $state<"signin" | "register">("signin");
  let username = $state("");
  let password = $state("");
  let confirmPassword = $state("");
  let error = $state("");
  let submitting = $state(false);

  function switchMode(newMode: "signin" | "register") {
    mode = newMode;
    error = "";
    password = "";
    confirmPassword = "";
  }

  async function handleSubmit(e?: Event) {
    if (e) e.preventDefault();
    error = "";

    const trimmedUser = username.trim();
    if (!trimmedUser) {
      error = "Please enter a username";
      return;
    }

    if (mode === "register") {
      if (trimmedUser.length < 3) {
        error = "Username must be at least 3 characters";
        return;
      }
      if (password.length < 6) {
        error = "Password must be at least 6 characters";
        return;
      }
      if (password !== confirmPassword) {
        error = "Passwords do not match";
        return;
      }

      submitting = true;
      try {
        const res = await register(trimmedUser, password);
        if (!res.ok) {
          error = res.error ?? "Registration failed";
        }
      } catch (err) {
        error = err instanceof Error ? err.message : "Network error during registration";
      } finally {
        submitting = false;
      }
    } else {
      submitting = true;
      try {
        const res = await login(trimmedUser, password);
        if (!res.ok) {
          error = res.error ?? "Invalid username or password";
        }
      } catch (err) {
        error = err instanceof Error ? err.message : "Network error during sign in";
      } finally {
        submitting = false;
      }
    }
  }

  async function handleGuestDemo() {
    error = "";
    submitting = true;
    try {
      const res = await continueAsGuest();
      if (!res.ok) {
        // Fallback to legacy demo credentials
        const fallback = await login("demo", "demo123");
        if (!fallback.ok) {
          error = fallback.error ?? "Failed to start demo session";
        }
      }
    } catch (err) {
      error = err instanceof Error ? err.message : "Network error starting demo session";
    } finally {
      submitting = false;
    }
  }
</script>

<div class="login-wrap">
  <div class="login-card">
    <div class="brand-row">
      <img src="/cardly-icon.svg" alt="" class="brand-icon" />
      <h1>Cardly</h1>
    </div>
    <p class="subtitle">Open-source spaced repetition web client & sync server.</p>

    <!-- Tab navigation -->
    <div class="tabs-nav" role="tablist">
      <button
        type="button"
        role="tab"
        aria-selected={mode === "signin"}
        class="tab-btn"
        class:active={mode === "signin"}
        onclick={() => switchMode("signin")}
      >
        Log In
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === "register"}
        class="tab-btn"
        class:active={mode === "register"}
        onclick={() => switchMode("register")}
      >
        Create Account
      </button>
    </div>

    {#if error}
      <div class="error-box" role="alert">{error}</div>
    {/if}

    <form onsubmit={handleSubmit}>
      <label class="field">
        <span>Username</span>
        <input
          class="anki-input"
          type="text"
          bind:value={username}
          autocomplete="username"
          placeholder={mode === "register" ? "e.g. alex_student" : "Enter your username"}
          required
        />
      </label>

      <label class="field">
        <span>Password</span>
        <input
          class="anki-input"
          type="password"
          bind:value={password}
          autocomplete={mode === "register" ? "new-password" : "current-password"}
          placeholder={mode === "register" ? "Min. 6 characters" : "Enter your password"}
          required
        />
      </label>

      {#if mode === "register"}
        <label class="field">
          <span>Confirm Password</span>
          <input
            class="anki-input"
            type="password"
            bind:value={confirmPassword}
            autocomplete="new-password"
            placeholder="Re-type your password"
            required
          />
        </label>
      {/if}

      <button type="submit" class="anki-btn primary submit-btn" disabled={submitting}>
        {#if submitting}
          {mode === "register" ? "Creating account..." : "Signing in..."}
        {:else}
          {mode === "register" ? "Create Account" : "Sign In"}
        {/if}
      </button>
    </form>

    <div class="divider">
      <span>or explore instantly</span>
    </div>

    <button
      type="button"
      class="anki-btn demo-btn"
      disabled={submitting}
      onclick={handleGuestDemo}
    >
      ⚡ Try Live Demo / Continue as Guest
    </button>
    <p class="demo-caption">
      Experience flashcards, AI generation, and study sessions with sample decks without registration.
    </p>
  </div>
</div>

<style>
  .login-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: var(--canvas);
  }
  .login-card {
    width: 100%;
    max-width: 410px;
    background: var(--canvas-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius-medium);
    padding: 28px;
    box-shadow: 0 10px 32px rgba(0, 0, 0, 0.12);
  }
  .brand-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 4px;
  }
  .brand-icon {
    width: 32px;
    height: 32px;
  }
  h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .subtitle {
    margin: 0 0 16px;
    font-size: 13.5px;
    color: var(--fg-subtle);
    line-height: 1.4;
  }
  .tabs-nav {
    display: flex;
    gap: 4px;
    background: var(--canvas-subtle, rgba(125, 125, 125, 0.1));
    padding: 4px;
    border-radius: var(--border-radius);
    margin-bottom: 18px;
  }
  .tab-btn {
    flex: 1;
    background: transparent;
    border: none;
    padding: 7px 12px;
    border-radius: var(--border-radius-small, 4px);
    font-size: 13.5px;
    font-weight: 600;
    color: var(--fg-subtle);
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .tab-btn.active {
    background: var(--canvas-elevated);
    color: var(--fg);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.08);
  }
  .tab-btn:hover:not(.active) {
    color: var(--fg);
  }
  .error-box {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid var(--accent-danger);
    color: var(--accent-danger);
    border-radius: var(--border-radius);
    padding: 8px 12px;
    font-size: 13px;
    margin-bottom: 14px;
    line-height: 1.4;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
    font-size: 13px;
    font-weight: 500;
  }
  .submit-btn {
    width: 100%;
    margin-top: 6px;
    padding: 10px 14px;
    font-size: 14px;
    font-weight: 600;
  }
  .divider {
    display: flex;
    align-items: center;
    text-align: center;
    margin: 20px 0 14px;
    color: var(--fg-muted, #888);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .divider::before,
  .divider::after {
    content: "";
    flex: 1;
    border-bottom: 1px solid var(--border-subtle);
  }
  .divider span {
    padding: 0 10px;
  }
  .demo-btn {
    width: 100%;
    padding: 10px 14px;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--accent-info, #0075ca);
    background: rgba(0, 117, 202, 0.08);
    border: 1px solid rgba(0, 117, 202, 0.35);
    border-radius: var(--border-radius);
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .demo-btn:hover {
    background: rgba(0, 117, 202, 0.15);
    border-color: rgba(0, 117, 202, 0.6);
  }
  .demo-caption {
    margin: 8px 0 0;
    font-size: 11.5px;
    color: var(--fg-subtle);
    text-align: center;
    line-height: 1.35;
  }
</style>
