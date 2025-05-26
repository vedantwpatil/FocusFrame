<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let isRecording = $state(false);
  let recordingName = $state("");
  let recordedFiles = $state<string[]>([]);

  async function startRecording(event: Event) {
    event.preventDefault();
    if (!recordingName) {
      alert("Please enter a name for the recording");
      return;
    }
    isRecording = true;
    // TODO: Implement recording start
  }

  async function stopRecording() {
    isRecording = false;
    // TODO: Implement recording stop
  }

  async function editVideo(filename: string) {
    // TODO: Implement video editing
  }
</script>

<main class="container">
  <h1>Focus Frame</h1>
  <p class="description">High-quality screen recording with smart zoom effects</p>

  <div class="recording-controls">
    {#if !isRecording}
      <form class="start-form" on:submit={startRecording}>
        <input 
          type="text" 
          placeholder="Enter recording name..."
          bind:value={recordingName}
          class="recording-input"
        />
        <button type="submit" class="record-button">
          Start Recording
        </button>
      </form>
    {:else}
      <div class="recording-status">
        <span class="recording-indicator"></span>
        Recording: {recordingName}
        <button class="stop-button" on:click={stopRecording}>
          Stop Recording
        </button>
      </div>
    {/if}
  </div>

  <div class="recordings-list">
    <h2>Recorded Videos</h2>
    {#if recordedFiles.length === 0}
      <p class="no-recordings">No recordings yet</p>
    {:else}
      <ul>
        {#each recordedFiles as file}
          <li class="recording-item">
            <span>{file}</span>
            <div class="actions">
              <button on:click={() => editVideo(file)}>Edit</button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</main>

<style>
  .container {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
  }

  h1 {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
    color: #2f2f2f;
  }

  .description {
    color: #666;
    margin-bottom: 2rem;
  }

  .recording-controls {
    background: #fff;
    padding: 1.5rem;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    margin-bottom: 2rem;
  }

  .start-form {
    display: flex;
    gap: 1rem;
  }

  .recording-input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 1rem;
  }

  .record-button {
    background: #ff3e00;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    transition: background-color 0.2s;
  }

  .record-button:hover {
    background: #e63600;
  }

  .stop-button {
    background: #dc3545;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
  }

  .recording-status {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .recording-indicator {
    width: 12px;
    height: 12px;
    background: #dc3545;
    border-radius: 50%;
    animation: pulse 1.5s infinite;
  }

  .recordings-list {
    background: #fff;
    padding: 1.5rem;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .recording-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid #eee;
  }

  .recording-item:last-child {
    border-bottom: none;
  }

  .actions button {
    background: #4a5568;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .no-recordings {
    color: #666;
    text-align: center;
    padding: 2rem;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
    100% {
      opacity: 1;
    }
  }

  @media (prefers-color-scheme: dark) {
    h1 {
      color: #f6f6f6;
    }

    .description {
      color: #ccc;
    }

    .recording-controls,
    .recordings-list {
      background: #2f2f2f;
    }

    .recording-input {
      background: #1f1f1f;
      border-color: #444;
      color: #fff;
    }

    .recording-item {
      border-bottom-color: #444;
    }

    .no-recordings {
      color: #ccc;
    }
  }
</style>