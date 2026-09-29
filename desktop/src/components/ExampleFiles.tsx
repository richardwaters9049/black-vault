import { FileCode2, FileText, GripVertical, ShieldAlert } from 'lucide-react';

// Built-in examples are in-memory records, not filesystem paths. Tauri's
// native drop event therefore reports paths=[] for these internal drags.
let activeExampleDrag: string | null = null;
export function getActiveExampleDrag(): string | null { return activeExampleDrag; }
export const EXAMPLE_DRAG_TYPE = 'application/x-black-vault-example';
export const examples = [
  { id: 'notes', name: 'example-notes.txt', detail: 'Plain text · passes', icon: FileText },
  { id: 'csv', name: 'example-records.csv', detail: 'CSV records · passes', icon: FileText },
  { id: 'sql', name: 'example-query.sql', detail: 'SQL text · passes', icon: FileCode2 },
  { id: 'blocked', name: 'example-script.sh', detail: 'Harmless script · intentionally rejected', icon: ShieldAlert },
] as const;

export default function ExampleFiles({ onSelect }: { onSelect: (id: string) => void }) {
  return <section aria-label="Example Files" className="mt-3 space-y-2 border-t border-blue-400/10 pt-3">
    <p className="px-2 text-[11px] leading-5 text-slate-400">Click a sample to stage it, or drag it into Encryption. These are built-in, offline examples; no code is executed.</p>
    {examples.map(sample => <button type="button" key={sample.id} draggable onClick={() => onSelect(sample.id)}
      onDragStart={event => { activeExampleDrag = sample.id; console.info('[vault-drop][ui] example drag started', { id: sample.id }); event.dataTransfer.effectAllowed = 'copy'; event.dataTransfer.setData(EXAMPLE_DRAG_TYPE, sample.id); event.dataTransfer.setData('text/plain', 'black-vault-example:' + sample.id); }}
      onDragEnd={() => { console.info('[vault-drop][ui] example drag ended', { id: sample.id }); window.setTimeout(() => { if (activeExampleDrag === sample.id) activeExampleDrag = null; }, 100); }}
      className="flex w-full cursor-grab items-start gap-2 rounded-lg border border-blue-400/10 bg-[#101b2c] p-2 text-left transition-colors hover:border-sky-400/50 hover:bg-blue-500/10 active:cursor-grabbing"
      title={'Click to select or drag ' + sample.name + ' into Encryption'}>
      <GripVertical size={14} className="mt-0.5 shrink-0 text-blue-400" />
      <sample.icon size={14} className="mt-0.5 shrink-0 text-sky-300" />
      <div className="min-w-0"><p className="break-all text-[11px] text-slate-200">{sample.name}</p><p className="mt-1 text-[10px] leading-4 text-slate-500">{sample.detail}</p></div>
    </button>)}
  </section>;
}
