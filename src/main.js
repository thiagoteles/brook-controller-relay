/**
 * Brook Controller Relay — Main Application
 *
 * Handles Tauri IPC, controller visualization, key remapping,
 * profile management, and relay control.
 */

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ── Application State ────────────────────────────────────────────────

/** @type {{ active_profile: string, device: { vid: string, pid: string, seize: boolean }, profiles: Record<string, { buttons: Record<string, { primary: string, secondary: string | null, label: string }>, dpad: { up: string, down: string, left: string, right: string } }> }} */
let config = null;

let relayRunning = false;
let injectionActive = true;
let deviceConnected = false;

// ── macOS keycode ↔ key name mapping ─────────────────────────────────
// Maps JavaScript `event.code` values to the macOS key names used by our backend.
const JS_CODE_TO_KEY_NAME = {
  KeyA: 'A', KeyB: 'B', KeyC: 'C', KeyD: 'D', KeyE: 'E', KeyF: 'F',
  KeyG: 'G', KeyH: 'H', KeyI: 'I', KeyJ: 'J', KeyK: 'K', KeyL: 'L',
  KeyM: 'M', KeyN: 'N', KeyO: 'O', KeyP: 'P', KeyQ: 'Q', KeyR: 'R',
  KeyS: 'S', KeyT: 'T', KeyU: 'U', KeyV: 'V', KeyW: 'W', KeyX: 'X',
  KeyY: 'Y', KeyZ: 'Z',
  Digit0: '0', Digit1: '1', Digit2: '2', Digit3: '3', Digit4: '4',
  Digit5: '5', Digit6: '6', Digit7: '7', Digit8: '8', Digit9: '9',
  Space: 'Space', Enter: 'Return', Tab: 'Tab', Escape: 'Escape',
  Backspace: 'Delete', Minus: 'Minus', Equal: 'Equal',
  BracketLeft: 'LeftBracket', BracketRight: 'RightBracket',
  Semicolon: 'Semicolon', Quote: 'Quote', Comma: 'Comma',
  Period: 'Period', Slash: 'Slash', Backslash: 'Backslash',
  Backquote: 'Grave',
  ArrowUp: 'UpArrow', ArrowDown: 'DownArrow',
  ArrowLeft: 'LeftArrow', ArrowRight: 'RightArrow',
  F1: 'F1', F2: 'F2', F3: 'F3', F4: 'F4', F5: 'F5', F6: 'F6',
  F7: 'F7', F8: 'F8', F9: 'F9', F10: 'F10', F11: 'F11', F12: 'F12',
};

// Display names for keys (user-friendly labels)
const KEY_DISPLAY = {
  Space: '␣', Return: '⏎', Tab: '⇥', Escape: '⎋', Delete: '⌫',
  UpArrow: '↑', DownArrow: '↓', LeftArrow: '←', RightArrow: '→',
  None: '—',
};

/** @param {string} keyName */
function getKeyDisplay(keyName) {
  if (!keyName || keyName === 'None') return '—';
  return KEY_DISPLAY[keyName] || keyName;
}

// ── DOM References ───────────────────────────────────────────────────

const $ = (/** @type {string} */ sel) => document.querySelector(sel);
const $$ = (/** @type {string} */ sel) => document.querySelectorAll(sel);

const statusIndicator = /** @type {HTMLElement} */ ($('#status-indicator'));
const statusText = /** @type {HTMLElement} */ ($('#status-text'));
const lastEventEl = /** @type {HTMLElement} */ ($('#last-event'));
const deviceNameEl = /** @type {HTMLElement} */ ($('#device-name'));
const profileSelect = /** @type {HTMLSelectElement} */ ($('#profile-select'));
const relayBtn = /** @type {HTMLButtonElement} */ ($('#btn-relay'));
const deviceSelect = /** @type {HTMLSelectElement} */ ($('#device-select'));
const toggleSeize = /** @type {HTMLElement} */ ($('#toggle-seize'));
const toggleInjection = /** @type {HTMLElement} */ ($('#toggle-injection'));
const toggleAutostart = /** @type {HTMLElement} */ ($('#toggle-autostart'));

// Modal
const modalOverlay = /** @type {HTMLElement} */ ($('#modal-overlay'));
const modalTitle = /** @type {HTMLElement} */ ($('#modal-title'));
const modalLabel = /** @type {HTMLInputElement} */ ($('#modal-label'));
const primaryKeyBox = /** @type {HTMLElement} */ ($('#primary-key-box'));
const secondaryKeyBox = /** @type {HTMLElement} */ ($('#secondary-key-box'));
const modalClose = /** @type {HTMLButtonElement} */ ($('#modal-close'));
const modalCancel = /** @type {HTMLButtonElement} */ ($('#modal-cancel'));
const modalSave = /** @type {HTMLButtonElement} */ ($('#modal-save'));
const modalClear = /** @type {HTMLButtonElement} */ ($('#modal-clear'));

// ── Initialization ───────────────────────────────────────────────────

async function init() {
  try {
    config = await invoke('get_config');
  } catch (err) {
    console.error('Failed to load config:', err);
    return;
  }

  updateUI();
  await refreshDevices();
  setupListeners();
  setupTauriEvents();

  // If auto_start is on, the backend already started the relay — sync UI state
  if (config.auto_start) {
    relayRunning = true;
    relayBtn.textContent = '■ Stop Relay';
    relayBtn.className = 'btn relay-btn stop';
    setLastEvent('Auto-started relay');
    setStatus('Listening...', false);
  }
}

// ── UI Updates ───────────────────────────────────────────────────────

function updateUI() {
  if (!config) return;

  // Profiles dropdown
  profileSelect.innerHTML = '';
  for (const name of Object.keys(config.profiles)) {
    const opt = document.createElement('option');
    opt.value = name;
    opt.textContent = name;
    opt.selected = name === config.active_profile;
    profileSelect.appendChild(opt);
  }

  // Device settings
  toggleSeize.classList.toggle('active', config.device.seize);
  toggleSeize.setAttribute('aria-checked', String(config.device.seize));

  // Auto-start toggle
  toggleAutostart.classList.toggle('active', config.auto_start || false);
  toggleAutostart.setAttribute('aria-checked', String(config.auto_start || false));

  // Button labels
  updateButtonLabels();
}

function updateButtonLabels() {
  if (!config) return;
  const profile = config.profiles[config.active_profile];
  if (!profile) return;

  // Action buttons (btn1-btn8) and meta buttons (btn9, btn10, btn13)
  const allBtnIds = ['btn1','btn2','btn3','btn4','btn5','btn6','btn7','btn8','btn9','btn10','btn11','btn12','btn13','btn14','btn15'];

  for (const btnId of allBtnIds) {
    const mapping = profile.buttons[btnId];
    if (!mapping) continue;

    const keyEl = $(`#key-${btnId}`);
    if (keyEl) {
      keyEl.textContent = getKeyDisplay(mapping.primary);
    }

    const secEl = $(`#sec-${btnId}`);
    if (secEl) {
      if (mapping.secondary && mapping.secondary !== 'None') {
        secEl.textContent = `+${getKeyDisplay(mapping.secondary)}`;
        secEl.classList.remove('hidden');
      } else {
        secEl.textContent = '';
        secEl.classList.add('hidden');
      }
    }

    // Update button label
    const labelEl = document.querySelector(`#arcade-${btnId} .btn-label`);
    if (labelEl && mapping.label) {
      // For main buttons, keep the physical button symbol prefix
      const arcadeEl = $(`#arcade-${btnId}`);
      if (arcadeEl && arcadeEl.classList.contains('arcade-btn')) {
        // Physical symbol is in the original HTML, just update label text
      }
    }
  }
}

function setStatus(text, connected) {
  statusText.textContent = text;
  statusIndicator.className = `status-indicator ${connected ? 'connected' : 'disconnected'}`;
}

function setLastEvent(text) {
  lastEventEl.textContent = text;
}

// ── Custom Dialog (replaces prompt/confirm/alert blocked in Tauri webview) ──

const dialogOverlay = /** @type {HTMLElement} */ ($('#dialog-overlay'));
const dialogTitle = /** @type {HTMLElement} */ ($('#dialog-title'));
const dialogMessage = /** @type {HTMLElement} */ ($('#dialog-message'));
const dialogInput = /** @type {HTMLInputElement} */ ($('#dialog-input'));
const dialogOk = /** @type {HTMLButtonElement} */ ($('#dialog-ok'));
const dialogCancel = /** @type {HTMLButtonElement} */ ($('#dialog-cancel'));
const dialogClose = /** @type {HTMLButtonElement} */ ($('#dialog-close'));

/**
 * Show a prompt dialog with an input field.
 * @param {string} title
 * @param {string} message
 * @param {string} [defaultValue]
 * @returns {Promise<string | null>} The entered value, or null if cancelled.
 */
function showPrompt(title, message, defaultValue = '') {
  return new Promise((resolve) => {
    dialogTitle.textContent = title;
    dialogMessage.textContent = message;
    dialogInput.style.display = 'block';
    dialogInput.value = defaultValue;
    dialogOk.textContent = 'OK';
    dialogOk.className = 'btn btn-primary';
    dialogOverlay.classList.add('visible');
    setTimeout(() => dialogInput.focus(), 50);

    const cleanup = () => {
      dialogOverlay.classList.remove('visible');
      dialogOk.onclick = null;
      dialogCancel.onclick = null;
      dialogClose.onclick = null;
      dialogInput.onkeydown = null;
    };

    dialogOk.onclick = () => { cleanup(); resolve(dialogInput.value.trim() || null); };
    dialogCancel.onclick = () => { cleanup(); resolve(null); };
    dialogClose.onclick = () => { cleanup(); resolve(null); };
    dialogInput.onkeydown = (e) => {
      if (e.key === 'Enter') { cleanup(); resolve(dialogInput.value.trim() || null); }
      if (e.key === 'Escape') { cleanup(); resolve(null); }
    };
  });
}

/**
 * Show a confirm dialog.
 * @param {string} title
 * @param {string} message
 * @param {string} [okLabel]
 * @returns {Promise<boolean>}
 */
function showConfirm(title, message, okLabel = 'Delete') {
  return new Promise((resolve) => {
    dialogTitle.textContent = title;
    dialogMessage.textContent = message;
    dialogInput.style.display = 'none';
    dialogOk.textContent = okLabel;
    dialogOk.className = 'btn btn-danger';
    dialogOverlay.classList.add('visible');

    const cleanup = () => {
      dialogOverlay.classList.remove('visible');
      dialogOk.onclick = null;
      dialogCancel.onclick = null;
      dialogClose.onclick = null;
    };

    dialogOk.onclick = () => { cleanup(); resolve(true); };
    dialogCancel.onclick = () => { cleanup(); resolve(false); };
    dialogClose.onclick = () => { cleanup(); resolve(false); };
  });
}

// ── Device Picker ────────────────────────────────────────────────────

async function refreshDevices() {
  try {
    /** @type {{ name: string, vid: string, pid: string }[]} */
    const devices = await invoke('list_devices');

    deviceSelect.innerHTML = '';

    if (devices.length === 0) {
      const opt = document.createElement('option');
      opt.value = '';
      opt.textContent = 'No devices found';
      opt.disabled = true;
      opt.selected = true;
      deviceSelect.appendChild(opt);
      return;
    }

    const currentKey = config ? `${config.device.vid}:${config.device.pid}` : '';
    let foundCurrent = false;

    for (const dev of devices) {
      const opt = document.createElement('option');
      opt.value = `${dev.vid}:${dev.pid}`;
      opt.textContent = `${dev.name} (${dev.vid}:${dev.pid})`;
      if (opt.value === currentKey) {
        opt.selected = true;
        foundCurrent = true;
      }
      deviceSelect.appendChild(opt);
    }

    // If the saved device wasn't found, add it as a placeholder
    if (!foundCurrent && config) {
      const opt = document.createElement('option');
      opt.value = currentKey;
      opt.textContent = `Saved device (${currentKey}) — not connected`;
      opt.selected = true;
      deviceSelect.insertBefore(opt, deviceSelect.firstChild);
    }

    setLastEvent(`Found ${devices.length} device${devices.length !== 1 ? 's' : ''}`);
  } catch (err) {
    console.error('Failed to list devices:', err);
    setLastEvent('Failed to scan devices');
  }
}

// ── Event Listeners ────────────────────────────────────────────────────────────────────

function setupListeners() {
  // Profile management
  profileSelect.addEventListener('change', onProfileChange);
  $('#btn-save-profile').addEventListener('click', onSaveProfile);
  $('#btn-rename-profile').addEventListener('click', onRenameProfile);
  $('#btn-new-profile').addEventListener('click', onNewProfile);
  $('#btn-delete-profile').addEventListener('click', onDeleteProfile);

  // Relay control
  relayBtn.addEventListener('click', onToggleRelay);

  // Toggles
  toggleSeize.addEventListener('click', () => {
    const active = !toggleSeize.classList.contains('active');
    toggleSeize.classList.toggle('active', active);
    toggleSeize.setAttribute('aria-checked', String(active));
    if (config) config.device.seize = active;
    saveConfigDebounced();
  });

  toggleInjection.addEventListener('click', () => {
    injectionActive = !toggleInjection.classList.contains('active');
    toggleInjection.classList.toggle('active', injectionActive);
    toggleInjection.setAttribute('aria-checked', String(injectionActive));
    if (relayRunning) {
      invoke('set_relay_active', { active: injectionActive }).catch(console.error);
    }
  });

  toggleAutostart.addEventListener('click', () => {
    const active = !toggleAutostart.classList.contains('active');
    toggleAutostart.classList.toggle('active', active);
    toggleAutostart.setAttribute('aria-checked', String(active));
    if (config) config.auto_start = active;
    saveConfigDebounced();
  });

  // Device picker
  deviceSelect.addEventListener('change', () => {
    const selected = deviceSelect.value;
    if (!selected || !config) return;
    const [vid, pid] = selected.split(':');
    config.device.vid = vid;
    config.device.pid = pid;
    saveConfigDebounced();
    const selectedOption = deviceSelect.options[deviceSelect.selectedIndex];
    setLastEvent(`Selected: ${selectedOption.textContent}`);
  });

  $('#btn-refresh-devices').addEventListener('click', refreshDevices);

  // Arcade buttons → open key picker
  $$('.arcade-btn, .meta-btn').forEach(el => {
    el.addEventListener('click', () => {
      const btnId = el.getAttribute('data-btn');
      if (btnId) openKeyPicker(btnId);
    });
  });

  // Modal
  modalClose.addEventListener('click', closeModal);
  modalCancel.addEventListener('click', closeModal);
  modalSave.addEventListener('click', onModalSave);
  modalClear.addEventListener('click', onModalClear);

  // Click outside modal to close
  modalOverlay.addEventListener('click', (e) => {
    if (e.target === modalOverlay) closeModal();
  });

  // Escape to close modal
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modalOverlay.classList.contains('visible')) {
      closeModal();
    }
  });
}

// ── Tauri Event Listeners ────────────────────────────────────────────

async function setupTauriEvents() {
  await listen('button-pressed', (event) => {
    /** @type {{ button: number, pressed: boolean, label: string }} */
    const data = event.payload;
    const btnId = `btn${data.button}`;
    const el = $(`#arcade-${btnId}`);
    if (el) {
      el.classList.toggle('active', data.pressed);
    }

    if (data.pressed) {
      setLastEvent(`Button ${data.button} pressed`);
    }
  });

  await listen('hat-changed', (event) => {
    /** @type {{ direction: number, keys: string[] }} */
    const data = event.payload;

    // Clear all d-pad buttons
    $$('.dpad-btn[data-dir]').forEach(el => el.classList.remove('active'));

    // Activate pressed directions
    for (const dir of data.keys) {
      const el = $(`#dpad-${dir}`);
      if (el) el.classList.add('active');
    }

    if (data.keys.length > 0) {
      setLastEvent(`D-Pad: ${data.keys.join('+')}`);
    }
  });

  await listen('device-status', (event) => {
    /** @type {{ name: string, connected: boolean }} */
    const data = event.payload;
    deviceConnected = data.connected;

    if (data.connected) {
      setStatus('Connected', true);
      deviceNameEl.textContent = data.name || 'Controller';
      setLastEvent('Device connected');
    } else {
      setStatus('Disconnected', false);
      deviceNameEl.textContent = 'No device';
      setLastEvent('Device disconnected');

      // Clear all button states
      $$('.arcade-btn, .meta-btn').forEach(el => el.classList.remove('active'));
      $$('.dpad-btn[data-dir]').forEach(el => el.classList.remove('active'));
    }
  });
}

// ── Relay Control ────────────────────────────────────────────────────

async function onToggleRelay() {
  if (relayRunning) {
    try {
      await invoke('stop_relay');
      relayRunning = false;
      relayBtn.textContent = '▶ Start Relay';
      relayBtn.className = 'btn relay-btn start';
      setLastEvent('Relay stopped');
    } catch (err) {
      console.error('Failed to stop relay:', err);
      setLastEvent(`Error: ${err}`);
    }
  } else {
    try {
      // Save current settings before starting
      await saveConfig();
      await invoke('start_relay', { relayActive: injectionActive });
      relayRunning = true;
      relayBtn.textContent = '■ Stop Relay';
      relayBtn.className = 'btn relay-btn stop';
      setLastEvent('Relay started — waiting for controller...');
      setStatus('Listening...', false);
    } catch (err) {
      console.error('Failed to start relay:', err);
      setLastEvent(`Error: ${err}`);
    }
  }
}

// ── Profile Management ───────────────────────────────────────────────

async function onProfileChange() {
  if (!config) return;
  config.active_profile = profileSelect.value;
  await saveConfig();
  updateButtonLabels();
  setLastEvent(`Switched to profile: ${config.active_profile}`);

  // Update live mappings if relay is running
  if (relayRunning) {
    try {
      await invoke('stop_relay');
      await invoke('start_relay', { relayActive: injectionActive });
    } catch (err) {
      console.error('Failed to update relay mappings:', err);
    }
  }
}

async function onSaveProfile() {
  if (!config) return;
  try {
    await saveConfig();
    setLastEvent(`Profile "${config.active_profile}" saved`);
  } catch (err) {
    console.error('Failed to save profile:', err);
    setLastEvent(`Error saving: ${err}`);
  }
}

async function onNewProfile() {
  const name = await showPrompt('New Profile', 'Enter a name for the new profile:');
  if (!name || !config) return;

  if (config.profiles[name]) {
    await showConfirm('Profile Exists', `A profile named "${name}" already exists.`, 'OK');
    return;
  }

  // Clone current profile
  const currentProfile = config.profiles[config.active_profile];
  config.profiles[name] = JSON.parse(JSON.stringify(currentProfile));
  config.active_profile = name;

  await saveConfig();
  updateUI();
  setLastEvent(`Created profile: ${name}`);
}

async function onRenameProfile() {
  if (!config) return;
  const oldName = config.active_profile;
  const newName = await showPrompt('Rename Profile', `Rename "${oldName}" to:`, oldName);
  if (!newName || newName === oldName) return;

  if (config.profiles[newName]) {
    await showConfirm('Profile Exists', `A profile named "${newName}" already exists.`, 'OK');
    return;
  }

  config.profiles[newName] = config.profiles[oldName];
  delete config.profiles[oldName];
  config.active_profile = newName;

  await saveConfig();
  updateUI();
  setLastEvent(`Renamed profile: ${oldName} → ${newName}`);
}

async function onDeleteProfile() {
  if (!config) return;
  if (Object.keys(config.profiles).length <= 1) {
    await showConfirm('Cannot Delete', 'You cannot delete the last profile.', 'OK');
    return;
  }

  const confirmed = await showConfirm('Delete Profile', `Delete profile "${config.active_profile}"? This cannot be undone.`);
  if (!confirmed) return;

  try {
    await invoke('delete_profile', { name: config.active_profile });
    delete config.profiles[config.active_profile];
    config.active_profile = Object.keys(config.profiles)[0];
    await saveConfig();
    updateUI();
    setLastEvent('Profile deleted');
  } catch (err) {
    console.error('Failed to delete profile:', err);
    setLastEvent(`Error: ${err}`);
  }
}

// ── Key Picker Modal ─────────────────────────────────────────────────

/** @type {string | null} */
let currentPickerBtn = null;
/** @type {'primary' | 'secondary' | null} */
let listeningFor = null;
let pickerPrimaryKey = 'None';
let pickerSecondaryKey = 'None';

function openKeyPicker(btnId) {
  if (!config) return;
  const profile = config.profiles[config.active_profile];
  if (!profile) return;

  currentPickerBtn = btnId;
  const mapping = profile.buttons[btnId];

  modalTitle.textContent = `Remap ${mapping?.label || btnId}`;
  modalLabel.value = mapping?.label || '';
  pickerPrimaryKey = mapping?.primary || 'None';
  pickerSecondaryKey = mapping?.secondary || 'None';

  primaryKeyBox.textContent = getKeyDisplay(pickerPrimaryKey);
  primaryKeyBox.classList.remove('listening');
  secondaryKeyBox.textContent = getKeyDisplay(pickerSecondaryKey);
  secondaryKeyBox.classList.remove('listening');

  listeningFor = null;
  modalOverlay.classList.add('visible');

  // Set up key listening
  primaryKeyBox.onclick = () => startListening('primary');
  secondaryKeyBox.onclick = () => startListening('secondary');
}

function startListening(/** @type {'primary' | 'secondary'} */ which) {
  listeningFor = which;
  primaryKeyBox.classList.toggle('listening', which === 'primary');
  secondaryKeyBox.classList.toggle('listening', which === 'secondary');

  if (which === 'primary') {
    primaryKeyBox.textContent = 'Press a key...';
  } else {
    secondaryKeyBox.textContent = 'Press a key...';
  }

  // Listen for the next keydown
  const handler = (/** @type {KeyboardEvent} */ e) => {
    e.preventDefault();
    e.stopPropagation();

    const keyName = JS_CODE_TO_KEY_NAME[e.code];
    if (!keyName) {
      // Unknown key
      return;
    }

    if (which === 'primary') {
      pickerPrimaryKey = keyName;
      primaryKeyBox.textContent = getKeyDisplay(keyName);
      primaryKeyBox.classList.remove('listening');
    } else {
      pickerSecondaryKey = keyName;
      secondaryKeyBox.textContent = getKeyDisplay(keyName);
      secondaryKeyBox.classList.remove('listening');
    }

    listeningFor = null;
    document.removeEventListener('keydown', handler, true);
  };

  document.addEventListener('keydown', handler, true);
}

function closeModal() {
  modalOverlay.classList.remove('visible');
  listeningFor = null;
  currentPickerBtn = null;
}

async function onModalSave() {
  if (!config || !currentPickerBtn) return;
  const profile = config.profiles[config.active_profile];
  if (!profile) return;

  const btnId = currentPickerBtn;

  if (!profile.buttons[btnId]) {
    profile.buttons[btnId] = { primary: 'None', secondary: null, label: btnId };
  }

  profile.buttons[btnId].primary = pickerPrimaryKey;
  profile.buttons[btnId].secondary = pickerSecondaryKey === 'None' ? null : pickerSecondaryKey;
  profile.buttons[btnId].label = modalLabel.value || profile.buttons[btnId].label;

  await saveConfig();
  updateButtonLabels();
  closeModal();
  setLastEvent(`Remapped ${btnId} → ${getKeyDisplay(pickerPrimaryKey)}`);
}

function onModalClear() {
  pickerPrimaryKey = 'None';
  pickerSecondaryKey = 'None';
  primaryKeyBox.textContent = '—';
  secondaryKeyBox.textContent = '—';
  primaryKeyBox.classList.remove('listening');
  secondaryKeyBox.classList.remove('listening');
  listeningFor = null;
}

// ── Config Persistence ───────────────────────────────────────────────

/** @type {ReturnType<typeof setTimeout> | null} */
let saveTimeout = null;

function saveConfigDebounced() {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(() => saveConfig(), 500);
}

async function saveConfig() {
  if (!config) return;
  try {
    await invoke('save_config_cmd', { newConfig: config });
  } catch (err) {
    console.error('Failed to save config:', err);
  }
}

// ── Assets directory cleanup ─────────────────────────────────────────

// Remove scaffold assets directory if it exists
const assetsDir = document.querySelector('link[rel="icon"]');
if (assetsDir) assetsDir.remove();

// ── Boot ─────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', init);
