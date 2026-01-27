# Frontend Development Guide - Crypto Community Platform

## Overview
Frontend development guide for crypto-focused communication platform using modern React ecosystem with Tauri.

## ✅ CURRENT IMPLEMENTATION STATUS

### Design System (FULLY IMPLEMENTED)
- **Glass Morphism UI**: Enhanced frosted glass effects with 16px backdrop blur throughout interface
- **Crypto Price Ticker**: Animated scrolling ticker with BTC/ETH brand colors and JetBrains Mono font
- **Navigation System**: Complete nav bar with active states (Dashboard, Portfolio, Chat, Markets)
- **Chat Interface**: Full messaging system with sent/received bubble styles and crypto addresses
- **Button Variants**: All 4 types implemented (Primary, Secondary, Success, Danger) with hover states
- **Form Components**: Enhanced inputs with focus states, select dropdowns, crypto-specific placeholders
- **Status Indicators**: Online/Offline/Warning badges with proper sage/stone/amber colors
- **Loading States**: Animated pulse placeholders for async operations
- **Typography**: Inter + JetBrains Mono fonts with crypto-optimized hierarchy
- **Color System**: Premium crypto palette (canvas, primary, sage, amber, coral, stone, market colors)
- **Animations**: Smooth transitions, hover scales, ticker animation, fade-in effects
- **Responsive Design**: Mobile-first approach with proper breakpoints
- **Accessibility**: Focus rings, proper contrast, WCAG compliance ready

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

## Recommended Tech Stack

### UI & Styling
- **Tailwind CSS** - Utility-first CSS framework
- **Headless UI** - Accessible, unstyled UI components
- **Framer Motion** - Animation library for React

### State & Data Management
- **Zustand** - Lightweight state management
- **TanStack Query** - Server state management & caching
- **React Hook Form** - Performant form handling

## State Management

```bash
npm install zustand
```

```typescript
import { create } from 'zustand';

// Example for crypto platform
interface AppState {
    user: User | null;
    activeChannel: Channel | null;
    messages: Message[];
    setUser: (user: User) => void;
    setActiveChannel: (channel: Channel) => void;
    addMessage: (message: Message) => void;
}

const useStore = create<AppState>((set) => ({
    user: null,
    activeChannel: null,
    messages: [],
    setUser: (user) => set({ user }),
    setActiveChannel: (channel) => set({ activeChannel: channel }),
    addMessage: (message) => set((state) => ({
        messages: [...state.messages, message]
    })),
}));

// Usage in component
function ChatHeader() {
    const { activeChannel, user } = useStore();
    return (
        <div className="flex items-center justify-between p-4">
            <h1>{activeChannel?.name || 'Select Channel'}</h1>
            <span>{user?.username}</span>
        </div>
    );
}
```

## Form Handling with React Hook Form

```typescript
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';

const messageSchema = z.object({
    content: z.string().min(1, 'Message cannot be empty').max(1000),
    channel: z.string().uuid(),
});

type MessageForm = z.infer<typeof messageSchema>;

function MessageInput() {
    const { register, handleSubmit, reset, formState: { errors } } = useForm<MessageForm>({
        resolver: zodResolver(messageSchema)
    });

    const onSubmit = (data: MessageForm) => {
        // Send message via Tauri IPC
        invoke('send_message', data);
        reset();
    };

    return (
        <form onSubmit={handleSubmit(onSubmit)} className="flex gap-2">
            <input
                {...register('content')}
                placeholder="Type a message..."
                className="flex-1 p-2 border rounded"
            />
            <button type="submit" className="px-4 py-2 bg-blue-500 text-white rounded">
                Send
            </button>
            {errors.content && <p className="text-red-500">{errors.content.message}</p>}
        </form>
    );
}
```

## Data Fetching with TanStack Query

```typescript
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';

// Fetch channels
function useChannels() {
    return useQuery({
        queryKey: ['channels'],
        queryFn: () => invoke<Channel[]>('get_channels'),
        staleTime: 5 * 60 * 1000, // 5 minutes
    });
}

// Send message mutation
function useSendMessage() {
    const queryClient = useQueryClient();

    return useMutation({
        mutationFn: (message: NewMessage) => invoke('send_message', message),
        onSuccess: () => {
            // Invalidate and refetch messages
            queryClient.invalidateQueries({ queryKey: ['messages'] });
        },
    });
}

// Usage in component
function ChannelList() {
    const { data: channels, isLoading, error } = useChannels();

    if (isLoading) return <div>Loading channels...</div>;
    if (error) return <div>Error loading channels</div>;

    return (
        <div className="space-y-2">
            {channels?.map(channel => (
                <div key={channel.id} className="p-2 hover:bg-gray-100 rounded">
                    {channel.name}
                </div>
            ))}
        </div>
    );
}
```
