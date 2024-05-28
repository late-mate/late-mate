import { assertEl, elById } from './asserts.ts';

const textarea = elById('textarea', 'latency-demo-textarea');
const textareaWrap = elById('div', 'latency-demo-textarea-wrap');

let currentDelay = 0;

const PRINTABLE_CHARACTERS =
  '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#$%^&*()-_+=[]\\{}|;\':",./<>?£ ';

textarea.disabled = false;

function syncTextareaHeight() {
  // see https://css-tricks.com/the-cleanest-trick-for-autogrowing-textareas/
  textareaWrap.dataset['replicatedValue'] = textarea.value;
}

textarea.addEventListener('keydown', (evt) => {
  if (currentDelay > 0) {
    evt.preventDefault();

    setTimeout(() => {
      switch (evt.key) {
        case 'Backspace':
          textarea.value = textarea.value.slice(0, -1);
          break;
        case 'Enter':
          textarea.value += '\n';
          break;
        case 'Meta':
        case 'Ctrl':
        case 'Shift':
          break;
        default:
          if (PRINTABLE_CHARACTERS.indexOf(evt.key) >= 0) {
            textarea.value += evt.key;
          }
      }
      // by this point, the content is updated, so it's ok to sync
      syncTextareaHeight();
    }, currentDelay);
  }
  // unfortunately, with no delay I can't sync here because the native event
  // hasn't changed the value yet
});

textarea.addEventListener('input', () => {
  // this is the only case I need to do this manually
  if (currentDelay == 0) {
    syncTextareaHeight();
  }
});

// delay slider

const delaySlider = elById('input', 'latency-demo-delay');
const delayValue = elById('div', 'latency-demo-delay-value');

function syncDelayValue() {
  const newDelay = parseInt(delaySlider.value);
  if (newDelay !== currentDelay) {
    // todo: animate
    textarea.value = '';
    syncTextareaHeight();

    currentDelay = newDelay;

    if (currentDelay > 0) {
      delayValue.innerText = `browser + ${delaySlider.value}ms`;
    } else {
      delayValue.innerText = 'browser-native';
    }
  }
}

delaySlider.addEventListener('input', (evt) => {
  const target = assertEl('input', evt.target);
  if (target !== delaySlider) {
    throw new Error(`Expected ${target} to be ${delaySlider}`);
  }
  syncDelayValue();
});

syncDelayValue();
