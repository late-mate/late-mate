import { h, render } from '/libs/preact.mjs'
import { useState, useEffect } from '/libs/hooks.js'
import htm from '/libs/htm.mjs'
const html = htm.bind(h)
import { signal } from '/libs/signals.mjs'
const { invoke } = window.__TAURI__.tauri

const typing_key = signal({value: 'X', state: 'idle'})
const erase_key = signal({value: 'Backspace', state: 'idle'})
const testing = signal({value: false, state: 'idle'})

function Key(props) {
    const { signal } = props
    if (signal.value.state === 'recording') {
        return html`<div class='key noselect recording'>
                        <div>Press any key...</div>
                    </div>`
    } else if (signal.value.state === 'setting') {
        return html`<div class='key noselect setting'>
                        <div>Saving...</div>
                    </div>`
    } else {
        const onClick = (e) => {
            signal.value = {...signal.value, state: 'recording'}
            e.stopPropagation();
        }
        return html`<div class='key noselect' onClick=${onClick}>
                        <div>${signal.value.value}</div>
                    </div>`
    }
}

async function onKeyDown(e){
    // console.log(e)
    if (e.shiftKey || e.ctrlKey || e.metaKey) {
        return;
    }

    const signal = typing_key.value.state == 'recording' ? typing_key
                 : erase_key.value.state == 'recording' ? erase_key
                 : null;

    if (signal === null) {
        return;
    }

    if (e.key != 'Escape') {
        const key = e.key == ' ' ? 'Space'
          : e.key.length == 1 ? e.key.toUpperCase()
          : e.key;
        signal.value = {...signal.value, state: 'setting' }
        await invoke('set_typing_key', { value: key })
        signal.value = {value: key, state: 'idle'}
    } else {
        signal.value = {...signal.value, state: 'idle'}
    }
}

function mouseClick(e) {
    const signal = typing_key.value.state == 'recording' ? typing_key
                 : erase_key.value.state == 'recording' ? erase_key
                 : null;
    if (signal) {
        signal.value = {...signal.value, state: 'idle'}
    }
}

function ButtonTesting(props) {
    const clz = testing.value.value === props.value && testing.value.state === 'idle' ? 'pressed' : ''
    const onClick = async (e) => {
        testing.value = {value: props.value, state: 'setting'}
        await invoke('set_testing', { value: props.value })
        testing.value = {...testing.value, state: 'idle'}
    }
    const text = props.value && testing.value.value && testing.value.state === 'idle' ? 'Started'
               : props.value && testing.value.value && testing.value.state === 'setting' ? 'Starting...'
               : props.value && !testing.value.value ? 'Start'
               : !props.value && !testing.value.value && testing.value.state === 'idle' ? 'Stopped'
               : !props.value && !testing.value.value && testing.value.state === 'setting' ? 'Stopping...'
               : 'Stop'
    return html`<button class=${clz} onClick=${onClick}>${text}</button>`
}

export function SetupTab(props) {
    useEffect(() => {
        window.addEventListener('keydown', onKeyDown, true)
        window.addEventListener('click', mouseClick, false)
        return () => {
            window.removeEventListener('keydown', onKeyDown, true)
            window.removeEventListener('click', mouseClick, false)
        }
    }, [])

    return html`<div class='panel setup'>
      <div>Typing key</div>
      <div><${Key} signal='${typing_key}' /></div>
      
      <div>Erase key</div>
      <div><${Key} signal='${erase_key}' /></div>
      
      <div>Test mode</div>
      <div class='testing'>
        <${ButtonTesting} value=${true} />
        <${ButtonTesting} value=${false} />
      </div>  
    </div>`
}
