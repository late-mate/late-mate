import { assertDefined, elById } from '../asserts.ts';

const originalColour = '#f2f2f2';
const highlightColour = 'oklch(62% 0.4 220)';

const explanationObject = elById('object', 'explanation-svg');

const ARROW_CLASS_KEY = 'arrowClass';

const explanationToArrowClass = [
  ['explanation-1', 'arrow-1'],
  ['explanation-2', 'arrow-2'],
  ['explanation-3', 'arrow-3'],
  ['explanation-4', 'arrow-4'],
];

const explanations: HTMLLIElement[] = [];

explanationToArrowClass.forEach(([explanationId, arrowClass]) => {
  const explanation = elById('li', explanationId);
  explanation.dataset[ARROW_CLASS_KEY] = arrowClass;
  explanations.push(explanation);
});

function setSvgColour(explanation: HTMLLIElement, colour: string) {
  // it has to be deferred as we might not have the element loaded yet,
  // and <object> apparently doesn't fire 'load" properly
  const explanationSvg = assertDefined(explanationObject.contentDocument);

  const arrowClass = assertDefined(explanation.dataset[ARROW_CLASS_KEY]);

  const toStrokeSel = `.${arrowClass}.tostroke`;
  const toFillSel = `.${arrowClass}.fill`;

  explanationSvg
    .querySelectorAll(toStrokeSel)
    .forEach((el) => el.setAttribute('stroke', colour));

  explanationSvg
    .querySelectorAll(toFillSel)
    .forEach((el) => el.setAttribute('fill', colour));
}

function setActive(explanation: HTMLLIElement) {
  explanation.classList.add('active');
  setSvgColour(explanation, highlightColour);
}

function unsetActive(explanation: HTMLLIElement) {
  explanation.classList.remove('active');
  setSvgColour(explanation, originalColour);
}

function initHover() {
  explanations.forEach((explanation) => {
    explanation.addEventListener('mouseover', () => setActive(explanation));
    explanation.addEventListener('mouseout', () => unsetActive(explanation));
  });
}

function initIntersection() {
  let lastHighlight: HTMLLIElement | null = null;

  const handleIntersect = (
    entries: IntersectionObserverEntry[],
    _observer: IntersectionObserver,
  ) => {
    console.log(entries);

    let maxIntersection: number | null = null;
    let highlight: HTMLLIElement | null = null;

    entries.forEach((entry) => {
      if (entry.intersectionRatio > (maxIntersection ?? -1)) {
        maxIntersection = entry.intersectionRatio;
        highlight = entry.target as HTMLLIElement;
      }
    });

    if (highlight !== null) {
      if (lastHighlight === null) {
        setActive(highlight);
        lastHighlight = highlight;
      } else {
        if (highlight !== lastHighlight) {
          unsetActive(lastHighlight);

          setActive(highlight);
          lastHighlight = highlight;
        }
      }
    } else if (lastHighlight !== null) {
      unsetActive(lastHighlight);
    }
  };

  const observerOptions = {
    root: null,
    rootMargin: '-49% 0% -49% 0%',
    threshold: 0,
  };

  const observer = new IntersectionObserver(handleIntersect, observerOptions);

  explanations.forEach((e) => observer.observe(e));
}

export function initDiagram() {
  if (matchMedia('(hover: hover)').matches) {
    initHover();
  } else {
    initIntersection();
  }
}
