// Scrim Overlay - Main Application Logic

interface UIElement {
  id: string;
  role: string;
  label: string | null;
  rect: { x: number; y: number; width: number; height: number };
  actions: string[];
}

interface Hint {
  label: string;
  element: UIElement;
  el: HTMLElement;
}

// State
let activeHints: Hint[] = [];
let isOverlayActive = false;
let clickListenerAdded = false;
let currentInput = '';
let appWindow: any = null;

// Home row keys for hints
const HINT_CHARS = 'ASDFGHJKL';

/**
 * Generate hint labels for a list of elements
 */
function generateHints(elements: UIElement[]): Map<string, UIElement> {
  const hintMap = new Map<string, UIElement>();
  const base = HINT_CHARS.length;

  for (let i = 0; i < elements.length; i++) {
    let label = '';
    let num = i;

    do {
      label = HINT_CHARS[num % base] + label;
      num = Math.floor(num / base);
      if (num > 0) num--;
    } while (num >= 0);

    hintMap.set(label, elements[i]);
  }

  return hintMap;
}

/**
 * Render hint overlays on the screen
 */
function renderHints(hintMap: Map<string, UIElement>): void {
  const container = document.getElementById('hint-container')!;
  container.innerHTML = '';
  activeHints = [];

  hintMap.forEach((element, label) => {
    const hint = document.createElement('div');
    hint.className = 'hint';
    hint.textContent = label;
    hint.style.left = `${element.rect.x}px`;
    hint.style.top = `${element.rect.y}px`;
    hint.dataset.label = label;

    container.appendChild(hint);

    activeHints.push({
      label,
      element,
      el: hint,
    });
  });

  console.log(`Rendered ${activeHints.length} hints`);
}

/**
 * Clear all hints from the screen
 */
function clearHints(): void {
  const container = document.getElementById('hint-container')!;
  container.innerHTML = '';
  activeHints = [];
  currentInput = '';
}

/**
 * Filter hints based on current input
 */
function filterHints(input: string): void {
  const exactMatch = activeHints.find(h => h.label === input);
  if (exactMatch) {
    // Found exact match - activate the element
    activateHint(exactMatch);
    return;
  }

  // Filter hints that start with the input
  activeHints.forEach(hint => {
    if (hint.label.startsWith(input)) {
      hint.el.classList.remove('dimmed');
      const remaining = hint.label.slice(input.length);
      hint.el.textContent = input + remaining;
    } else {
      hint.el.classList.add('dimmed');
    }
  });
}

/**
 * Activate a hint (click the target element)
 */
async function activateHint(hint: Hint): Promise<void> {
  console.log(`Activating hint: ${hint.label}`, hint.element);

  // Highlight the hint
  hint.el.classList.add('active');

  try {
    // Perform the action via Tauri
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('perform_action', {
      elementId: hint.element.id,
      action: 'Press',
    });
    console.log('Action performed successfully');

    // Clear hints after activation
    setTimeout(() => {
      clearHints();
      deactivateOverlay();
    }, 100);
  } catch (error) {
    console.error('Failed to perform action:', error);
  }
}

/**
 * Activate the overlay and show hints
 */
async function activateOverlay(): Promise<void> {
  if (isOverlayActive) {
    // Toggle off if already active
    deactivateOverlay();
    return;
  }

  console.log('Activating overlay...');

  try {
    // Get UI elements from the backend
    const { invoke } = await import('@tauri-apps/api/core');
    const { getCurrentWindow } = await import('@tauri-apps/api/window');

    // Request window focus to ensure keyboard events work
    appWindow = getCurrentWindow();
    await appWindow.setFocus(true);
    console.log('Window focus requested');

    const elements = await invoke('get_elements', {
      filter: 'interactive',
    }) as UIElement[];

    console.log(`Found ${elements.length} interactive elements`);

    // Generate and render hints
    const hintMap = generateHints(elements);
    renderHints(hintMap);

    isOverlayActive = true;
    currentInput = '';

    // Enable click-through after hints are rendered
    await invoke('set_click_through', { ignore: true });
    console.log('Overlay activated, click-through enabled');
  } catch (error) {
    console.error('Failed to get elements:', error);
    // Check permissions again
    const { invoke: invoke2 } = await import('@tauri-apps/api/core');
    const hasPermission = await invoke2('check_permissions') as boolean;
    if (!hasPermission) {
      showPermissionPrompt();
    }
  }
}

/**
 * Deactivate the overlay
 */
function deactivateOverlay(): void {
  console.log('Deactivating overlay');
  clearHints();
  isOverlayActive = false;

  // Disable click-through so we can click the scrim-app window
  const { invoke } = require('@tauri-apps/api/core');
  invoke('set_click_through', { ignore: false }).catch(e => console.error(e));
}

/**
 * Show permission prompt if needed
 */
async function showPermissionPrompt(): Promise<void> {
  // Disable click-through so the button can be clicked
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_click_through', { ignore: false });
  } catch (e) {
    console.error('Failed to disable click-through:', e);
  }

  const prompt = document.createElement('div');
  prompt.id = 'permission-prompt';
  prompt.innerHTML = `
    <h2>Accessibility Permission Required</h2>
    <p>Scrim needs accessibility permissions to navigate your desktop.</p>
    <p>Please grant permission in System Settings > Privacy & Security > Accessibility</p>
    <button id="retry-btn">Retry</button>
    <button id="request-btn">Open System Settings</button>
  `;

  const retryBtn = prompt.querySelector('#retry-btn');
  retryBtn?.addEventListener('click', () => {
    location.reload();
  });

  const requestBtn = prompt.querySelector('#request-btn');
  requestBtn?.addEventListener('click', async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('request_permissions');
    } catch (e) {
      console.error('Failed to request permissions:', e);
    }
  });

  document.body.appendChild(prompt);
}

/**
 * Handle keyboard input
 */
function handleKeyDown(event: KeyboardEvent): void {
  console.log(`Keyboard event: key="${event.key}", isOverlayActive=${isOverlayActive}`);

  if (!isOverlayActive) return;

  const key = event.key.toUpperCase();
  console.log(`Processing key: ${key}, HINT_CHARS=${HINT_CHARS}, includes=${HINT_CHARS.includes(key)}`);

  // Escape to deactivate
  if (event.key === 'Escape') {
    deactivateOverlay();
    event.preventDefault();
    return;
  }

  // Check if key is a valid hint character
  if (HINT_CHARS.includes(key)) {
    currentInput += key;
    console.log(`Current input: "${currentInput}"`);
    filterHints(currentInput);
    event.preventDefault();
    event.stopPropagation();
  }
}

/**
 * Handle global keyboard events from backend (rdev)
 */
function handleGlobalKey(key: string): void {
  console.log(`Global key: ${key}, isOverlayActive=${isOverlayActive}`);

  if (!isOverlayActive) return;

  // Escape to deactivate
  if (key === 'Escape') {
    deactivateOverlay();
    return;
  }

  // Check if key is a valid hint character
  if (HINT_CHARS.includes(key)) {
    currentInput += key;
    console.log(`Current input: "${currentInput}" (global)`);
    filterHints(currentInput);
  }
}

/**
 * Initialize the application
 */
async function init(): Promise<void> {
  console.log('Initializing Scrim overlay...');

  try {
    // Check permissions first
    const { invoke } = await import('@tauri-apps/api/core');
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const { listen } = await import('@tauri-apps/api/event');
    const hasPermission = await invoke('check_permissions') as boolean;
    console.log('Has permission:', hasPermission);

    if (!hasPermission) {
      showPermissionPrompt();
      return;
    }

    console.log('Permissions granted! Starting hint display...');

    // Get window reference
    appWindow = getCurrentWindow();

    // Track window focus
    appWindow.onFocusChanged(({ payload: focused }: { payload: boolean }) => {
      console.log(`Window focus changed: focused=${focused}`);
    });

    // Listen for global keyboard events from backend
    await listen('keyboard-input', (event: { payload: string }) => {
      console.log(`Global keyboard event: ${event.payload}`);
      handleGlobalKey(event.payload);
    });

    // Also listen for local keyboard events (fallback)
    document.addEventListener('keydown', handleKeyDown);

    // Activate hints on load
    await activateOverlay();

    console.log('Scrum overlay initialized - Type hint characters (A, S, D...) to select elements');
    console.log('Global keyboard monitoring enabled - works regardless of window focus');
  } catch (error) {
    console.error('Failed to initialize:', error);
  }
}

// Start the application when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}

export {};
