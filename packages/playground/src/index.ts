import {run_tcl} from 'tanaid';

const inputEl = document.getElementById('input')! as HTMLTextAreaElement;
const outputEl = document.getElementById('output')! as HTMLPreElement;

inputEl.addEventListener('input', () => {
  const code = inputEl.value;
  try {
    const result = run_tcl(code);
    outputEl.innerText = result;
  } catch (e) {
    outputEl.innerText = String(e);
  }
});
