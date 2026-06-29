// Switch Configuration Tab Panels
function switchTab(tabName) {
  const tabs = document.querySelectorAll('.config-tab')
  const panels = document.querySelectorAll('.config-panel')

  tabs.forEach((tab) => {
    tab.classList.remove('active')
  })
  panels.forEach((panel) => {
    panel.classList.remove('active')
  })

  const activeTab = document.getElementById(`btn-tab-${tabName}`)
  const activePanel = document.getElementById(`panel-${tabName}`)

  if (activeTab && activePanel) {
    activeTab.classList.add('active')
    activePanel.classList.add('active')
  }
}

// Copy Command Box Text to Clipboard
async function copyText(elementId, buttonId) {
  const codeElement = document.getElementById(elementId)
  const buttonElement = document.getElementById(buttonId)

  if (!codeElement || !buttonElement) return

  const textToCopy = codeElement.innerText || codeElement.textContent

  try {
    await navigator.clipboard.writeText(textToCopy)

    // Save original button content
    const originalHTML = buttonElement.innerHTML

    // Update button to show visual success state
    buttonElement.innerHTML = '✅ Copied!'
    buttonElement.style.borderColor = 'var(--color-success)'
    buttonElement.style.color = 'var(--color-success)'

    setTimeout(() => {
      buttonElement.innerHTML = originalHTML
      buttonElement.style.borderColor = ''
      buttonElement.style.color = ''
    }, 2000)
  } catch (err) {
    console.error('Failed to copy text: ', err)
  }
}

// Fetch GitHub Releases
async function fetchReleases() {
  const stableLink = document.getElementById('stable-link')

  const prereleaseCard = document.getElementById('prerelease-card')
  const prereleaseLink = document.getElementById('prerelease-link')

  if (!stableLink || !prereleaseCard) return

  try {
    const response = await fetch('https://api.github.com/repos/rpr13/llama-herd/releases')
    if (!response.ok) throw new Error('Failed to fetch releases')
    const releases = await response.json()

    // Find latest stable (where prerelease is false)
    const stable = releases.find((r) => !r.prerelease)
    // Find latest pre-release (where prerelease is true)
    const pre = releases.find((r) => r.prerelease)

    if (stable) {
      const version = stable.name || stable.tag_name
      stableLink.innerHTML = `📦 <span class="visually-hidden">Download</span> Stable (${version})`
      stableLink.href = stable.html_url
    } else {
      stableLink.innerHTML = `📦 <span class="visually-hidden">Download</span> Stable (v1.0.16)`
      stableLink.href = 'https://github.com/rpr13/llama-herd/releases/latest'
    }

    if (pre) {
      const version = pre.name || pre.tag_name
      prereleaseLink.innerHTML = `🔬 <span class="visually-hidden">Download</span> Pre-release (${version})`
      prereleaseLink.href = pre.html_url
      prereleaseCard.classList.remove('disabled')
    } else {
      prereleaseLink.innerHTML = `🔬 <span class="visually-hidden">Download</span> Pre-release`
      prereleaseLink.removeAttribute('href')
      prereleaseLink.style.pointerEvents = 'none'
      prereleaseCard.classList.add('disabled')
    }
  } catch (err) {
    console.error('Error fetching releases:', err)
    // Fallback states
    stableLink.innerHTML = `📦 <span class="visually-hidden">Download</span> Stable (v1.0.16)`
    stableLink.href = 'https://github.com/rpr13/llama-herd/releases/latest'

    prereleaseLink.innerHTML = `🔬 <span class="visually-hidden">Download</span> Pre-release`
    prereleaseCard.classList.add('disabled')
  }
}

// Theme presets HTML templates
const themePresets = {
  cyberpunk: `<span class="toml-comment"># Custom TUI theme at ~/.config/llama-herd/theme.toml</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"cyan"</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"gray"</span>
<span class="toml-key">accent</span> = <span class="toml-value">"yellow"</span>
<span class="toml-key">success</span> = <span class="toml-value">"green"</span>
<span class="toml-key">error</span> = <span class="toml-value">"red"</span>
<span class="toml-key">selection</span> = <span class="toml-value">"magenta"</span>
<span class="toml-key">bg</span> = <span class="toml-value">"black"</span>
<span class="toml-key">fg</span> = <span class="toml-value">"white"</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"indexed(234)"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"indexed(234)"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"rounded"</span> <span class="toml-comment"># plain, rounded, double, thick</span>`,

  amber: `<span class="toml-comment"># Custom TUI theme at ~/.config/llama-herd/theme.toml</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#ffb000"</span>     <span class="toml-comment"># Amber</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#805800"</span>   <span class="toml-comment"># Dark Amber</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#ffcc00"</span>      <span class="toml-comment"># Bright Amber</span>
<span class="toml-key">success</span> = <span class="toml-value">"#ffb000"</span>
<span class="toml-key">error</span> = <span class="toml-value">"#ff3333"</span>       <span class="toml-comment"># Alert Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#ffb000"</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#000000"</span>          <span class="toml-comment"># Pure Black</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#ffb000"</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"indexed(0)"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"indexed(0)"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">false</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"thick"</span>   <span class="toml-comment"># retro borders</span>`,

  dracula: `<span class="toml-comment"># Custom TUI theme at ~/.config/llama-herd/theme.toml</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#bd93f9"</span>     <span class="toml-comment"># Purple</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#6272a4"</span>   <span class="toml-comment"># Comment Gray</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#f1fa8c"</span>      <span class="toml-comment"># Yellow</span>
<span class="toml-key">success</span> = <span class="toml-value">"#50fa7b"</span>     <span class="toml-comment"># Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#ff5555"</span>       <span class="toml-comment"># Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#44475a"</span>   <span class="toml-comment"># Current Line</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#282a36"</span>          <span class="toml-comment"># Background</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#f8f8f2"</span>          <span class="toml-comment"># Foreground</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#1e1f29"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#1e1f29"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"rounded"</span>`,

  nordic: `<span class="toml-comment"># Custom TUI theme at ~/.config/llama-herd/theme.toml</span>

<span class="toml-table">[palette]</span>
<span class="toml-key">primary</span> = <span class="toml-value">"#88c0d0"</span>     <span class="toml-comment"># Frost Teal</span>
<span class="toml-key">secondary</span> = <span class="toml-value">"#4c566a"</span>   <span class="toml-comment"># Slate Gray</span>
<span class="toml-key">accent</span> = <span class="toml-value">"#b48ead"</span>      <span class="toml-comment"># Frost Purple</span>
<span class="toml-key">success</span> = <span class="toml-value">"#a3be8c"</span>     <span class="toml-comment"># Frost Green</span>
<span class="toml-key">error</span> = <span class="toml-value">"#bf616a"</span>       <span class="toml-comment"># Frost Red</span>
<span class="toml-key">selection</span> = <span class="toml-value">"#434c5e"</span>   <span class="toml-comment"># Selection Dark</span>
<span class="toml-key">bg</span> = <span class="toml-value">"#2e3440"</span>          <span class="toml-comment"># Polar Night</span>
<span class="toml-key">fg</span> = <span class="toml-value">"#eceff4"</span>          <span class="toml-comment"># Snow Storm</span>
<span class="toml-key">header-bg</span> = <span class="toml-value">"#242933"</span>
<span class="toml-key">footer-bg</span> = <span class="toml-value">"#242933"</span>

<span class="toml-table">[ui]</span>
<span class="toml-key">show-emojis</span> = <span class="toml-value">true</span>
<span class="toml-key">border-type</span> = <span class="toml-value">"double"</span>`,
}

// Switch Theme Presets
function selectThemePreset(presetName) {
  const codeBlock = document.getElementById('theme-code-block')
  if (!codeBlock || !themePresets[presetName]) return

  codeBlock.innerHTML = themePresets[presetName]

  const buttons = document.querySelectorAll('.theme-preset-btn')
  buttons.forEach((btn) => btn.classList.remove('active'))

  const activeButton = document.getElementById(`btn-theme-${presetName}`)
  if (activeButton) {
    activeButton.classList.add('active')
  }
}

// Dynamically generate running llamas based on screen aspect ratio
function setupRunningLlamas() {
  const container = document.querySelector('.running-llamas-container')
  if (!container) return

  // Clear existing static llamas
  container.innerHTML = ''

  const width = window.innerWidth
  const height = window.innerHeight
  const aspectRatio = width / height

  // Calculate number of llamas based on aspect ratio (e.g. 16:9 ~ 1.78 aspect ratio yields 16 llamas)
  let numLlamas = Math.round(aspectRatio * 9)
  if (numLlamas < 4) numLlamas = 4
  if (numLlamas > 24) numLlamas = 24

  for (let i = 0; i < numLlamas; i++) {
    const llama = document.createElement('div')
    llama.className = 'running-llama'

    const size = 80 + Math.random() * 100
    const opacity = 0.35 + Math.random() * 0.5
    const runDuration = 12 + Math.random() * 20
    const gallopDuration = 0.4 + Math.random() * 0.6
    const runDelay = Math.random() * 25
    const gallopDelay = Math.random() * 0.4

    llama.style.width = `${size}px`
    llama.style.height = `${size}px`
    llama.style.opacity = opacity
    llama.style.animation = `run-across ${runDuration}s linear infinite, gallop ${gallopDuration}s ease-in-out infinite`
    llama.style.animationDelay = `${runDelay}s, ${gallopDelay}s`

    container.appendChild(llama)
  }
}

// Call on load and on resize
document.addEventListener('DOMContentLoaded', () => {
  fetchReleases()
  setupRunningLlamas()
})
window.addEventListener('resize', setupRunningLlamas)
window.addEventListener('orientationchange', () => {
  // A small timeout ensures screen dimensions have updated before recalculating
  setTimeout(setupRunningLlamas, 150)
})
