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
  const stableVersion = document.getElementById('stable-version')
  const stableDate = document.getElementById('stable-date')
  const stableLink = document.getElementById('stable-link')

  const prereleaseCard = document.getElementById('prerelease-card')
  const prereleaseVersion = document.getElementById('prerelease-version')
  const prereleaseDate = document.getElementById('prerelease-date')
  const prereleaseLink = document.getElementById('prerelease-link')

  if (!stableVersion || !prereleaseCard) return

  try {
    const response = await fetch('https://api.github.com/repos/rpr13/llama-herd/releases')
    if (!response.ok) throw new Error('Failed to fetch releases')
    const releases = await response.json()

    // Find latest stable (where prerelease is false)
    const stable = releases.find((r) => !r.prerelease)
    // Find latest pre-release (where prerelease is true)
    const pre = releases.find((r) => r.prerelease)

    if (stable) {
      stableVersion.textContent = stable.name || stable.tag_name
      const date = new Date(stable.published_at)
      stableDate.textContent = `Released on ${date.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' })}`
      stableLink.href = stable.html_url
    } else {
      stableVersion.textContent = 'v1.0.8' // fallback
      stableDate.textContent = 'Latest Stable'
    }

    if (pre) {
      prereleaseVersion.textContent = pre.name || pre.tag_name
      const date = new Date(pre.published_at)
      prereleaseDate.textContent = `Released on ${date.toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' })}`
      prereleaseLink.href = pre.html_url
      prereleaseCard.classList.remove('disabled')
    } else {
      prereleaseVersion.textContent = 'None available'
      prereleaseDate.textContent = 'No active pre-release'
      prereleaseLink.removeAttribute('href')
      prereleaseLink.style.pointerEvents = 'none'
      prereleaseCard.classList.add('disabled')
    }
  } catch (err) {
    console.error('Error fetching releases:', err)
    // Fallback states
    stableVersion.textContent = 'v1.0.8'
    stableDate.textContent = 'Latest Stable'

    prereleaseVersion.textContent = 'None available'
    prereleaseDate.textContent = 'No active pre-release'
    prereleaseCard.classList.add('disabled')
  }
}

// Call on load
document.addEventListener('DOMContentLoaded', fetchReleases)

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
