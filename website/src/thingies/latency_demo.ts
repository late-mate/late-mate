import { assertEl, elById } from '../asserts.ts';

const textarea = elById('textarea', 'latency-demo-textarea');
const textareaWrap = elById('div', 'latency-demo-textarea-wrap');

let currentDelay = 0;

const PRINTABLE_CHARACTERS =
  '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#$%^&*()-_+=[]\\{}|;\':",./<>?£ ';

function syncTextareaHeight() {
  // see https://css-tricks.com/the-cleanest-trick-for-autogrowing-textareas/
  textareaWrap.dataset['replicatedValue'] = textarea.value;
}

export function initTextarea() {
  textarea.disabled = false;

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
}

const delaySlider = elById('input', 'latency-demo-delay');
const delayValue = elById('div', 'latency-demo-delay-value');

let cleaningInProgress = false;

function cleanTextarea() {
  if (cleaningInProgress) {
    return;
  }

  if (textarea.value.trim() === '') {
    textarea.value = '';
  }

  cleaningInProgress = true;

  textarea.disabled = true;
  let breakpoints = [...textarea.value.matchAll(/\s+/g)].map(
    (match) => match.index,
  );

  if (breakpoints[0] !== 0) {
    breakpoints = [0, ...breakpoints];
  }

  // 16ms per frame
  // 300ms cleanup
  // => 16ms pause, up to 18 steps

  const step = Math.ceil(breakpoints.length / 18);
  if (step === 0) {
    throw new Error('unexpected 0 step');
  }

  const makeStep = () => {
    // last step might be shorter
    const currentStep = Math.min(breakpoints.length, step);

    const cutoff = breakpoints[breakpoints.length - currentStep];
    textarea.value = textarea.value.substring(0, cutoff);
    syncTextareaHeight();

    breakpoints.splice(-currentStep, currentStep);
  };

  const finalise = () => {
    textarea.disabled = false;
    cleaningInProgress = false;
  };

  makeStep();

  const handle = setInterval(() => {
    if (breakpoints.length > 0) {
      makeStep();
    } else {
      clearInterval(handle);
      finalise();
    }
  }, 16);
}

export function initSlider() {
  function syncDelayValue() {
    const newDelay = parseInt(delaySlider.value);
    if (newDelay !== currentDelay) {
      cleanTextarea();

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
}
