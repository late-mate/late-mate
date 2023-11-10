const { invoke } = window.__TAURI__.tauri
import { h, Component, render } from '/libs/preact.mjs'
import { useState } from '/libs/hooks.js'
import htm from '/libs/htm.mjs'
const html = htm.bind(h)

import { SetupTab } from "/setup.js"
import { MeasureTab } from "/measure.js"

function TabBar(props) {
  return html`<div class='tabbar'>
                ${['Setup', 'Measure'].map(tab => 
                  html`<div onClick=${() => props.setTab(tab)}
                         class=${'tab noselect' + (props.tab == tab ? ' selected' : '')}>
                         ${tab}
                       </div>`
                )}
              </div>`;
}

function App () {
  const [tab, setTab] = useState('Setup')
  return html`<div class='app'>
                <${TabBar} tab=${tab} setTab=${setTab}/>
                ${tab == 'Setup' ? html`<${SetupTab} />` : html`<${MeasureTab} />`}
              </div>`;
}

function mount() {
  render(html`<${App} />`, document.body)
}

mount()
