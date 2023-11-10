import { h, render } from '/libs/preact.mjs'
import { useState, useEffect } from '/libs/hooks.js'
import htm from '/libs/htm.mjs'
const html = htm.bind(h)
const { emit, listen } = window.__TAURI__.event

const unlisten = await listen('measurement', (event) => {
    console.log(event)
  // event.event is the event name (useful if you want to use a single callback fn for multiple event types)
  // event.payload is the payload object
})

export function MeasureTab(props) {
    return h('div', {class: 'panel'}, 'Measure')
}