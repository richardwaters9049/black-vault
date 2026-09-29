import { useState } from 'react';
import { ChevronDown, FileCode2, FileJson2, FileText, GripVertical, LockKeyhole, ShieldAlert } from 'lucide-react';

// Built-in examples are in-memory records, not filesystem paths. Tauri's
// native drop event therefore reports paths=[] for these internal drags.
let activeExampleDrag: string | null = null;
let activeDecryptDrag: string | null = null;
export function getActiveDecryptDrag(): string | null { return activeDecryptDrag; }
export function getActiveExampleDrag(): string | null { return activeExampleDrag; }
export const EXAMPLE_DRAG_TYPE = 'application/x-black-vault-example';
export const examples = [
  { id: 'notes', name: 'example-notes.txt', detail: 'Plain text · passes', icon: FileText },
  { id: 'csv', name: 'example-records.csv', detail: 'CSV records · passes', icon: FileText },
  { id: 'sql', name: 'example-query.sql', detail: 'SQL text · passes', icon: FileCode2 },
  { id: 'json', name: 'example-settings.json', detail: 'JSON settings · passes', icon: FileJson2 },
  { id: 'markdown', name: 'example-readme.md', detail: 'Markdown document · passes', icon: FileText },
  { id: 'log', name: 'example-audit.log', detail: 'Audit log · passes', icon: FileText },
  { id: 'blocked', name: 'example-script.sh', detail: 'Harmless script · intentionally rejected', icon: ShieldAlert },
] as const;

export default function ExampleFiles({ onSelect, decryptFiles = [], onDecryptSelect }: {
  onSelect: (id: string) => void;
  decryptFiles?: Array<{ id: string; name: string }>;
  onDecryptSelect?: (id: string) => void;
}) {
  const [encryptOpen, setEncryptOpen] = useState(false);
  const [decryptOpen, setDecryptOpen] = useState(false);
  return <section aria-label="Example Files" className="mt-3 space-y-2 border-t border-blue-400/10 pt-3">
    <p className="px-2 text-[11px] leading-5 text-slate-400">Click a sample to stage it, or drag it into Encryption. These are built-in, offline examples; no code is executed.</p>
    <button type="button" aria-expanded={encryptOpen} aria-controls="encrypt-examples-panel" onClick={() => setEncryptOpen(value => !value)}
      className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-left text-[11px] font-medium text-sky-300 transition-colors hover:bg-blue-500/10">
      <FileText size={13}/> Encrypt files <span className="ml-auto font-mono text-[10px] text-slate-500">{examples.length}</span>
      <ChevronDown size={14} className={"transition-transform duration-300 ease-out " + (encryptOpen ? "rotate-180" : "")}/>
    </button>
    <div id="encrypt-examples-panel" aria-hidden={!encryptOpen} inert={!encryptOpen}
      className={"grid transition-[grid-template-rows,opacity] duration-300 ease-out motion-reduce:transition-none " + (encryptOpen ? "grid-rows-[1fr] opacity-100" : "pointer-events-none grid-rows-[0fr] opacity-0")}>
      <div className="min-h-0 overflow-hidden"><div className="space-y-2">
    {examples.map(sample => <button type="button" key={sample.id} draggable onClick={() => onSelect(sample.id)}
      onDragStart={event => { activeExampleDrag = sample.id; console.info('[vault-drop][ui] example drag started', { id: sample.id }); event.dataTransfer.effectAllowed = 'copy'; event.dataTransfer.setData(EXAMPLE_DRAG_TYPE, sample.id); event.dataTransfer.setData('text/plain', 'black-vault-example:' + sample.id); }}
      onDragEnd={() => { console.info('[vault-drop][ui] example drag ended', { id: sample.id }); window.setTimeout(() => { if (activeExampleDrag === sample.id) activeExampleDrag = null; }, 100); }}
      className="flex w-full cursor-grab items-start gap-2 rounded-lg border border-blue-400/10 bg-[#101b2c] p-2 text-left transition-colors hover:border-sky-400/50 hover:bg-blue-500/10 active:cursor-grabbing"
      title={'Click to select or drag ' + sample.name + ' into Encryption'}>
      <GripVertical size={14} className="mt-0.5 shrink-0 text-blue-400" />
      <sample.icon size={14} className="mt-0.5 shrink-0 text-sky-300" />
      <div className="min-w-0"><p className="break-all text-[11px] text-slate-200">{sample.name}</p><p className="mt-1 text-[10px] leading-4 text-slate-500">{sample.detail}</p></div>
    </button>)}
      </div></div>
    </div>
    <button type="button" aria-expanded={decryptOpen} aria-controls="decrypt-examples-panel" onClick={() => setDecryptOpen(value => !value)}
      className="flex w-full items-center gap-2 rounded-lg border-t border-blue-400/10 px-2 py-2 text-left text-[11px] font-medium text-sky-300 transition-colors hover:bg-blue-500/10">
      <LockKeyhole size={13}/> Decrypt files <span className="ml-auto font-mono text-[10px] text-slate-500">{decryptFiles.length}</span>
      <ChevronDown size={14} className={"transition-transform duration-300 ease-out " + (decryptOpen ? "rotate-180" : "")}/>
    </button>
    <div id="decrypt-examples-panel" aria-hidden={!decryptOpen} inert={!decryptOpen}
      className={"grid transition-[grid-template-rows,opacity] duration-300 ease-out motion-reduce:transition-none " + (decryptOpen ? "grid-rows-[1fr] opacity-100" : "pointer-events-none grid-rows-[0fr] opacity-0")}>
      <div className="min-h-0 overflow-hidden"><div className="space-y-2">
    <p className="px-2 text-[10px] leading-4 text-slate-500">Actual encrypted files in the unlocked vault. Encrypt an example first to add it here.</p>
    {decryptFiles.length === 0 ? <p className="px-2 text-[11px] text-slate-500">No encrypted files yet.</p> : decryptFiles.map(file =>
      <button key={file.id} type="button" draggable onClick={() => onDecryptSelect?.(file.id)}
        onDragStart={event => { activeDecryptDrag = file.id; console.info('[vault-drop][ui] encrypted file drag started', { id: file.id.slice(0, 8) }); event.dataTransfer.effectAllowed = 'copy'; event.dataTransfer.setData('application/x-black-vault-encrypted-file', file.id); event.dataTransfer.setData('text/plain', 'black-vault-file:' + file.id); }}
        onDragEnd={() => { window.setTimeout(() => { if (activeDecryptDrag === file.id) activeDecryptDrag = null; }, 100); }}
        className="flex w-full cursor-grab items-center gap-2 rounded-lg border border-blue-400/10 bg-[#101b2c] p-2 text-left text-[11px] text-slate-200 transition-colors hover:border-sky-400/50 hover:bg-blue-500/10 active:cursor-grabbing">
        <GripVertical size={14} className="shrink-0 text-blue-400"/><LockKeyhole size={14} className="shrink-0 text-sky-300"/><span className="min-w-0 break-all">{file.name}</span>
      </button>)}
      </div></div>
    </div>
  </section>;
}
