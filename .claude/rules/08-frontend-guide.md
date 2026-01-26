# Frontend Development Guide

## Structure

```
src/
├── App.tsx             # Main application component
├── App.css             # Application styles
├── main.tsx            # Entry point
├── vite-env.d.ts       # Vite type definitions
└── assets/             # Static assets
```

## React with Tauri

### Basic Component

```tsx
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function App() {
    const [result, setResult] = useState('');

    async function handleClick() {
        const greeting = await invoke<string>('greet', { name: 'User' });
        setResult(greeting);
    }

    return (
        <div>
            <button onClick={handleClick}>Greet</button>
            <p>{result}</p>
        </div>
    );
}

export default App;
```

## Tauri APIs

### Window Management

```typescript
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// Minimize
await appWindow.minimize();

// Maximize
await appWindow.maximize();

// Close
await appWindow.close();

// Set title
await appWindow.setTitle('New Title');
```

### File System

First, add the plugin:
```bash
npm run tauri add fs
```

```typescript
import { readTextFile, writeTextFile, BaseDirectory } from '@tauri-apps/plugin-fs';

// Read file
const content = await readTextFile('config.json', {
    baseDir: BaseDirectory.AppData
});

// Write file
await writeTextFile('config.json', JSON.stringify(data), {
    baseDir: BaseDirectory.AppData
});
```

### Dialogs

First, add the plugin:
```bash
npm run tauri add dialog
```

```typescript
import { open, save, message } from '@tauri-apps/plugin-dialog';

// Open file picker
const filePath = await open({
    multiple: false,
    filters: [{
        name: 'Text',
        extensions: ['txt', 'md']
    }]
});

// Save dialog
const savePath = await save({
    defaultPath: 'document.txt'
});

// Message box
await message('Operation completed!', { title: 'Success' });
```

### HTTP Requests

First, add the plugin:
```bash
npm run tauri add http
```

```typescript
import { fetch } from '@tauri-apps/plugin-http';

const response = await fetch('https://api.example.com/data', {
    method: 'GET',
    headers: {
        'Content-Type': 'application/json'
    }
});

const data = await response.json();
```

## Platform Detection

```typescript
import { platform } from '@tauri-apps/plugin-os';

const currentPlatform = await platform();

switch (currentPlatform) {
    case 'windows':
        // Windows-specific UI
        break;
    case 'macos':
        // macOS-specific UI
        break;
    case 'linux':
        // Linux-specific UI
        break;
    case 'android':
        // Android-specific UI
        break;
    case 'ios':
        // iOS-specific UI
        break;
}
```

## Responsive Design for Mobile

```css
/* Base styles for mobile-first */
.container {
    padding: 16px;
    font-size: 16px;
}

/* Tablet and larger */
@media (min-width: 768px) {
    .container {
        padding: 24px;
        max-width: 720px;
        margin: 0 auto;
    }
}

/* Desktop */
@media (min-width: 1024px) {
    .container {
        max-width: 960px;
    }
}

/* Safe areas for notched devices (iOS) */
.app {
    padding-top: env(safe-area-inset-top);
    padding-bottom: env(safe-area-inset-bottom);
    padding-left: env(safe-area-inset-left);
    padding-right: env(safe-area-inset-right);
}
```

## State Management

For larger applications, consider:

- **Zustand** - Lightweight state management
- **Jotai** - Atomic state management
- **Redux Toolkit** - Full-featured state management

```bash
npm install zustand
```

```typescript
import { create } from 'zustand';

interface AppState {
    count: number;
    increment: () => void;
}

const useStore = create<AppState>((set) => ({
    count: 0,
    increment: () => set((state) => ({ count: state.count + 1 })),
}));

function Counter() {
    const { count, increment } = useStore();
    return <button onClick={increment}>{count}</button>;
}
```
