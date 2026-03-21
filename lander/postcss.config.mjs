import path from 'path';
import { fileURLToPath } from 'node:url';

const landerRoot = path.dirname(fileURLToPath(import.meta.url));

const config = {
  plugins: {
    '@tailwindcss/postcss': {
      base: landerRoot,
    },
  },
};

export default config;
