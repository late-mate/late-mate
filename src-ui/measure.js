import { h, render } from '/libs/preact.mjs'
import { useState, useEffect } from '/libs/hooks.js'
import htm from '/libs/htm.mjs'
const html = htm.bind(h)

export function MeasureTab(props) {
    return html`<div class='panel'>Measure</div>`
}