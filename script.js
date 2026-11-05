// Switch Configuration Tab Panels
function switchTab(tabName) {
  const tabs = document.querySelectorAll(".config-tab");
  const panels = document.querySelectorAll(".config-panel");

  tabs.forEach((tab) => {
    tab.classList.remove("active");
  });
  panels.forEach((panel) => {
    panel.classList.remove("active");
  });

  const activeTab = document.getElementById(`btn-tab-${tabName}`);
  const activePanel = document.getElementById(`panel-${tabName}`);

  if (activeTab && activePanel) {
    activeTab.classList.add("active");
    activePanel.classList.add("active");
  }
}

// Copy Command Box Text to Clipboard
async function copyText(elementId, buttonId) {
  const codeElement = document.getElementById(elementId);
  const buttonElement = document.getElementById(buttonId);

  if (!codeElement || !buttonElement) return;

  const textToCopy = codeElement.innerText || codeElement.textContent;

  try {
    await navigator.clipboard.writeText(textToCopy);

    // Save original button content
    const originalHTML = buttonElement.innerHTML;

    // Update button to show visual success state
    buttonElement.innerHTML = "✅ Copied!";
    buttonElement.style.borderColor = "var(--color-success)";
    buttonElement.style.color = "var(--color-success)";

    setTimeout(() => {
      buttonElement.innerHTML = originalHTML;
      buttonElement.style.borderColor = "";
      buttonElement.style.color = "";
    }, 2000);
  } catch (err) {
    console.error("Failed to copy text: ", err);
  }
}

// Fetch GitHub Releases
async function fetchReleases() {
  const stableLink = document.getElementById("stable-link");

  const prereleaseCard = document.getElementById("prerelease-card");
  const prereleaseLink = document.getElementById("prerelease-link");

  if (!stableLink || !prereleaseCard) return;

  try {
    const response = await fetch(
      "https://api.github.com/repos/rpr13/llama-herd/releases",
    );
    if (!response.ok) throw new Error("Failed to fetch releases");
    const releases = await response.json();

    // Find latest stable (where prerelease is false)
    const stable = releases.find((r) => !r.prerelease);
    // Find latest pre-release (where prerelease is true)
    const pre = releases.find((r) => r.prerelease);

    if (stable) {
      const version = stable.name || stable.tag_name;
      stableLink.innerHTML = `📦 <span class="visually-hidden">Download</span> Stable (${version})`;
      stableLink.href = stable.html_url;
    } else {
      stableLink.innerHTML = `📦 <span class="visually-hidden">Download</span> Stable (v1.0.22)`;
      stableLink.href = "https://github.com/rpr13/llama-herd/releases/latest";
    }

    if (pre) {
      const version = pre.name || pre.tag_name;
      prereleaseLink.innerHTML = `🔬 <span class="visually-hidden">Download</span> Pre-release (${version})`;
      prereleaseLink.href = pre.html_url;
      prereleaseCard.classList.remove("disabled");
    } else {
      prereleaseLink.innerHTML = `🔬 <span class="visually-hidden">Download</span> Pre-release`;
      prereleaseLink.removeAttribute("href");
      prereleaseLink.style.pointerEvents = "none";
      prereleaseCard.classList.add("disabled");
    }
  } catch (err) {
    console.error("Error fetching releases:", err);
    // Fallback states
    stableLink.innerHTML = `📦 <span class="visually-hidden">Download</span> Stable (v1.0.22)`;
    stableLink.href = "https://github.com/rpr13/llama-herd/releases/latest";

    prereleaseLink.innerHTML = `🔬 <span class="visually-hidden">Download</span> Pre-release`;
    prereleaseCard.classList.add("disabled");
  }
}

// Theme presets HTML templates
const themePresets = {
  gruvbox: `<span class="toml-comment"># Built-in preset "gruvbox-dark" (MIT License)</span>
<span class="toml-comment"># Set in config.toml: theme = "gruvbox-dark"</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#83a598"</span>     <span class="toml-comment"># Aqua</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#928374"</span>   <span class="toml-comment"># Gray</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#fabd2f"</span>      <span class="toml-comment"># Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#b8bb26"</span>     <span class="toml-comment"># Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#fb4934"</span>       <span class="toml-comment"># Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#d3869b"</span>   <span class="toml-comment"># Purple</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#282828"</span>          <span class="toml-comment"># Dark0 (or reset via theme-transparent)</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#ebdbb2"</span>          <span class="toml-comment"># Light1</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#32302f"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#32302f"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"plain"</span>`,

  vscode: `<span class="toml-comment"># Built-in preset "vscode-dark" (MIT License)</span>
<span class="toml-comment"># Set in config.toml: theme = "vscode-dark"</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#007acc"</span>     <span class="toml-comment"># Blue</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#808080"</span>   <span class="toml-comment"># Gray</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#dcdcaa"</span>      <span class="toml-comment"># Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#6a9955"</span>     <span class="toml-comment"># Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#f44747"</span>       <span class="toml-comment"># Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#c586c0"</span>   <span class="toml-comment"># Violet</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#1e1e1e"</span>          <span class="toml-comment"># Dark BG (or reset via theme-transparent)</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#d4d4d4"</span>          <span class="toml-comment"># Light FG</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#252526"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#252526"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"plain"</span>`,

  intellij: `<span class="toml-comment"># Built-in preset "intellij-dark" (Apache 2.0 License)</span>
<span class="toml-comment"># Set in config.toml: theme = "intellij-dark"</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#299999"</span>     <span class="toml-comment"># Teal</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#808080"</span>   <span class="toml-comment"># Gray</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#bbb529"</span>      <span class="toml-comment"># Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#629755"</span>     <span class="toml-comment"># Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#ff6b68"</span>       <span class="toml-comment"># Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#9876aa"</span>   <span class="toml-comment"># Purple</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#2b2b2b"</span>          <span class="toml-comment"># Darcula Dark (or reset via theme-transparent)</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#a9b7c6"</span>          <span class="toml-comment"># Darcula FG</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#313335"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#313335"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"plain"</span>`,

  catppuccin: `<span class="toml-comment"># Built-in preset "catppuccin-mocha" (MIT License)</span>
<span class="toml-comment"># Set in config.toml: theme = "catppuccin-mocha"</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#89b4fa"</span>     <span class="toml-comment"># Blue</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#a6adc8"</span>   <span class="toml-comment"># Subtext0</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#f9e2af"</span>      <span class="toml-comment"># Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#a6e3a1"</span>     <span class="toml-comment"># Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#f38ba8"</span>       <span class="toml-comment"># Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#cba6f7"</span>   <span class="toml-comment"># Mauve</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#1e1e2e"</span>          <span class="toml-comment"># Base (or reset via theme-transparent)</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#cdd6f4"</span>          <span class="toml-comment"># Text</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#181825"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#181825"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"plain"</span>`,

  tokyonight: `<span class="toml-comment"># Built-in preset "tokyo-night" (MIT License)</span>
<span class="toml-comment"># Set in config.toml: theme = "tokyo-night"</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#7aa2f7"</span>     <span class="toml-comment"># Blue</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#565f89"</span>   <span class="toml-comment"># Comment</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#e0af68"</span>      <span class="toml-comment"># Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#9ece6a"</span>     <span class="toml-comment"># Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#f7768e"</span>       <span class="toml-comment"># Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#bb9af7"</span>   <span class="toml-comment"># Purple</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#1a1b26"</span>          <span class="toml-comment"># Storm Dark (or reset via theme-transparent)</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#c0caf5"</span>          <span class="toml-comment"># White</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#16161e"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#16161e"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"plain"</span>`,

  nord: `<span class="toml-comment"># Built-in preset "nord" (MIT License)</span>
<span class="toml-comment"># Set in config.toml: theme = "nord"</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#88c0d0"</span>     <span class="toml-comment"># Frost Teal</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#4c566a"</span>   <span class="toml-comment"># Polar Gray</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#ebcb8b"</span>      <span class="toml-comment"># Aurora Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#a3be8c"</span>     <span class="toml-comment"># Aurora Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#bf616a"</span>       <span class="toml-comment"># Aurora Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#b48ead"</span>   <span class="toml-comment"># Aurora Purple</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#2e3440"</span>          <span class="toml-comment"># Polar Night (or reset via theme-transparent)</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#eceff4"</span>          <span class="toml-comment"># Snow Storm</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#242933"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#242933"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"plain"</span>`,
};

// Switch Theme Presets
function selectThemePreset(presetName) {
  const codeBlock = document.getElementById("theme-code-block");
  if (!codeBlock || !themePresets[presetName]) return;

  codeBlock.innerHTML = themePresets[presetName];

  const buttons = document.querySelectorAll(".theme-preset-btn");
  buttons.forEach((btn) => btn.classList.remove("active"));

  const activeButton = document.getElementById(`btn-theme-${presetName}`);
  if (activeButton) {
    activeButton.classList.add("active");
  }
}

// Dynamically generate running llamas based on screen aspect ratio
function setupRunningLlamas() {
  const container = document.querySelector(".running-llamas-container");
  if (!container) return;

  // Clear existing static llamas
  container.innerHTML = "";

  const width = window.innerWidth;
  const height = window.innerHeight;
  const aspectRatio = width / height;

  // Calculate number of llamas based on aspect ratio (e.g. 16:9 ~ 1.78 aspect ratio yields 16 llamas)
  let numLlamas = Math.round(aspectRatio * 9);
  if (numLlamas < 4) numLlamas = 4;
  if (numLlamas > 24) numLlamas = 24;

  for (let i = 0; i < numLlamas; i++) {
    const llama = document.createElement("div");
    llama.className = "running-llama";

    const size = 80 + Math.random() * 100;
    const opacity = 0.35 + Math.random() * 0.5;
    const runDuration = 12 + Math.random() * 20;
    const gallopDuration = 0.4 + Math.random() * 0.6;
    const runDelay = Math.random() * 25;
    const gallopDelay = Math.random() * 0.4;

    llama.style.width = `${size}px`;
    llama.style.height = `${size}px`;
    llama.style.opacity = opacity;
    llama.style.animation = `run-across ${runDuration}s linear infinite, gallop ${gallopDuration}s ease-in-out infinite`;
    llama.style.animationDelay = `${runDelay}s, ${gallopDelay}s`;

    container.appendChild(llama);
  }
}

// Call on load and on resize
document.addEventListener("DOMContentLoaded", () => {
  fetchReleases();
  setupRunningLlamas();
});
window.addEventListener("resize", setupRunningLlamas);
window.addEventListener("orientationchange", () => {
  // A small timeout ensures screen dimensions have updated before recalculating
  setTimeout(setupRunningLlamas, 150);
});
