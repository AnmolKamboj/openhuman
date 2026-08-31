import { createRoot } from 'react-dom/client';
import { createRef } from 'react';
import './index.css';
import ThreadList from './features/conversations/threadList/ThreadList';

const threads = Array.from({ length: 12 }, (_, i) => ({
  id: `t${i}`,
  title: `Conversation about topic number ${i + 1}`,
})) as never;

const noop = () => {};

createRoot(document.getElementById('root')!).render(
  <div className="h-screen p-3">
    <div className="h-full w-64 rounded-2xl border border-line">
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
    </div>
  </div>
);
