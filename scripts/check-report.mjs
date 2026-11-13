// Dependency-free render/interaction smoke test against a minimal DOM contract.
// This checks JavaScript/data binding, not actual browser layout or accessibility.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';

const file = process.argv[2];
if (!file) throw new Error('Usage: node scripts/check-report.mjs <report.html>');
const html = fs.readFileSync(file, 'utf8');
const json = html.match(/<script type="application\/json" id="report-data">([\s\S]*?)<\/script>/)[1];
const script = html.match(/<script>\s*([\s\S]*?)<\/script>/)[1];
const elements = new Map();
class Element {
  constructor(id) { this.id=id; this.listeners={}; this.attributes={}; this.dataset={}; this.clientWidth=1100; this.html=''; this.textContent=''; }
  set innerHTML(value) { this.html=value; }
  get innerHTML() { return this.html; }
  addEventListener(name, callback) { this.listeners[name]=callback; }
  setAttribute(name, value) { this.attributes[name]=value; }
  querySelectorAll(selector) {
    assert.equal(selector, 'button');
    return [...this.html.matchAll(/data-profile="([^"]+)"/g)].map(match=>{
      const e = new Element(match[1]); e.dataset.profile=match[1]; return e;
    });
  }
}
for (const match of html.matchAll(/\bid="([^"]+)"/g)) {
  assert(!elements.has(match[1]), 'Duplicate ID: ' + match[1]);
  elements.set(match[1], new Element(match[1]));
}
elements.get('report-data').textContent=json;
let printCount=0;
const context={document:{getElementById(id){assert(elements.has(id),'Unknown DOM ID: '+id);return elements.get(id);}},
  window:{print(){printCount++;},addEventListener(){}},console,Date,Set,JSON,Math,Number,String,Object};
vm.runInNewContext(script, context, {filename:'report-script.js'});
const data=JSON.parse(json);
assert.equal(elements.get('stat-runs').textContent, data.trials.filter(t=>!t.prime&&t.status==='sampled'&&t.playback_evidence?.verified).length);
assert.equal((elements.get('field-body').innerHTML.match(/<tr>/g)||[]).length,12);
assert(elements.get('field-body').innerHTML.includes('Windows Media Player Legacy'));
assert(elements.get('field-body').innerHTML.includes('Windows Media Player'));
assert(elements.get('field-body').innerHTML.includes('Foxide'));
assert(elements.get('native-diagnostic-content').innerHTML.includes('not eligible for medals'));
assert(elements.get('results-body').innerHTML.includes('GPU') === false);
for(const profile of ['A','B','C','D']) {
  const button=new Element(profile);button.dataset.profile=profile;
  elements.get('profile-controls').listeners.click({target:{closest:()=>button}});
  assert(elements.get('profile-description').textContent.includes('PROFILE '+profile));
  assert(elements.get('timeline').innerHTML.includes('<svg'));
  assert(!/NaN|Infinity|undefined/.test(elements.get('timeline').innerHTML));
  for(const metric of data.metrics) {
    elements.get('metric').listeners.change({target:{value:metric.id}});
    const result=elements.get('podiums').innerHTML;
    assert.equal((result.match(/class="podium"/g)||[]).length,3);
    if(!metric.ranked) assert.equal((result.match(/Not awarded/g)||[]).length,3);
  }
}
elements.get('print-report').listeners.click();assert.equal(printCount,1);
assert(!/<script[^>]+src=/.test(html),'Report must work without remote scripts');
assert(!/fetch\(|XMLHttpRequest|WebSocket/.test(script),'Report should not make background network requests');
console.log('Report checks passed: 12 entrants, 4 media selectors, all metric podiums, offline data, native diagnostics and print action.');
