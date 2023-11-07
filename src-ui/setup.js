import { h, render } from '/libs/preact.mjs'
import { useState, useEffect } from '/libs/hooks.js'
import htm from '/libs/htm.mjs'
const html = htm.bind(h)
import { signal } from '/libs/signals.mjs'

const recording = signal(null)
const keys = signal(['X', 'Backspace'])

function Key(props) {
    const { recording, setRecording, k } = props
    return html`<div class='key noselect ${recording ? 'recording' : ''}'
                     onClick=${(e) => { setRecording(); e.stopPropagation(); }}>
      <div>${recording ? 'Press any key...' : k}</div>
    </div>`
}

export function SetupTab(props) {
     const keyPress = (e) => {
        // console.log(e)
        if (e.shiftKey || e.ctrlKey || e.metaKey) {
            return;
        }
        if (recording.value === null) {
            return;
        }
        if (e.key != 'Escape') {
            const key = e.key == ' ' ? 'Space'
              : e.key.length == 1 ? e.key.toUpperCase()
              : e.key;
            keys.value[recording.value] = key
        }
        recording.value = null
    }

    const mouseClick = (e) => {
        recording.value = null;
    }
    
    useEffect(() => {
        window.addEventListener('keydown', keyPress, true)
        window.addEventListener('click', mouseClick, false)
        return () => {
            window.removeEventListener('keydown', keyPress, true)
            window.removeEventListener('click', mouseClick, false)
        }
    }, [])

    return html`<div class='panel setup'>
      <div>Typing key</div>
      <div><${Key} k='${keys.value[0]}' recording=${recording.value === 0} setRecording=${() => recording.value = 0} /></div>
      <div>Erase key</div>
      <div><${Key} k='${keys.value[1]}' recording=${recording.value === 1} setRecording=${() => recording.value = 1} /></div>
    </div>`
}
