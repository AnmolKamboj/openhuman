import { createRoot } from 'react-dom/client';
import { createRef } from 'react';
import './index.css';
import { ThreadList } from './features/conversations/threadList/ThreadList';

const threads = Array.from({ length: 6 }, (_, i) => ({ id: `t${i}`, title: `Conversation ${i}` })) as never;
const noop = () => {};
const list = (
  <ThreadList
    threads={threads}
    selectedThreadId="t2"
    onCreateThread={noop}
    onSelectThread={noop}
    resolveTitle={(id: string) => `Conversation about topic ${id}`}
    onRequestDelete={noop}
    editingThreadId={null}
    editTitleValue=""
    editTitleInputRef={createRef<HTMLInputElement>()}
    onEditTitleValueChange={noop}
    onStartEditTitle={noop}
    onCommitTitle={noop}
    onCancelEditTitle={noop}
    onBlurTitle={noop}
  />
);

createRoot(document.getElementById('root')!).render(
  <div className="flex h-screen gap-4 p-3">
    <div className="h-72 w-64 rounded-2xl border border-line">{list}</div>
    <div className="dark h-72 w-64 rounded-2xl border border-line bg-surface-chrome" data-testid="dark-pane">
      {list}
    </div>
  </div>
);
