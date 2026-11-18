<script lang="ts">
  import { login } from "../lib/api";
  import { store } from "../lib/store.svelte";

  let username = $state("");
  let password = $state("");
  let error = $state("");
  let submitting = $state(false);

  async function handleSubmit(e?: Event) {
    if (e) e.preventDefault();
    error = "";
    submitting = true;
    try {
      const res = await login(username.trim(), password);
      if (!res.ok) {
        error = res.error ?? "Invalid username or password";
      }
    } catch (err) {
      error = err instanceof Error ? err.message : "Network error";
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
    <p class="subtitle">Sign in to your spaced repetition collection.</p>
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
          required
          autofocus
        />
      </label>
      <label class="field">
        <span>Password</span>
        <input
          class="anki-input"
          type="password"
          bind:value={password}
          autocomplete="current-password"
          required
        />
      </label>
      <button type="submit" class="anki-btn primary submit-btn" disabled={submitting}>
        {submitting ? "Signing in..." : "Sign In"}
      </button>
      {#if store.demoMode}
        <button
          type="button"
          class="anki-btn demo-btn"
          disabled={submitting}
          onclick={() => {
            username = "demo";
            password = "demo123";
            handleSubmit();
          }}
        >
          ⚡ Try Demo (demo / demo123)
        </button>
      {/if}
    </form>
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
    max-width: 390px;
    background: var(--canvas-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--border-radius-medium);
    padding: 28px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.1);
  }
  .brand-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }
  .brand-icon {
    width: 30px;
    height: 30px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
  }
  .subtitle {
    margin: 0 0 18px;
    font-size: 13.5px;
    color: var(--fg-subtle);
  }
  .error-box {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid var(--accent-danger);
    color: var(--accent-danger);
    border-radius: var(--border-radius);
    padding: 8px 10px;
    font-size: 13px;
    margin-bottom: 14px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 14px;
    font-size: 13px;
    font-weight: 500;
  }
  .submit-btn {
    width: 100%;
    margin-top: 4px;
    padding: 9px 14px;
  }
  .demo-btn {
    width: 100%;
    margin-top: 8px;
    padding: 8px 14px;
    color: var(--accent-info, #0075ca);
    background: rgba(0, 117, 202, 0.08);
    border: 1px dashed rgba(0, 117, 202, 0.4);
    font-weight: 600;
  }
  .demo-btn:hover {
    background: rgba(0, 117, 202, 0.15);
  }
</style>
