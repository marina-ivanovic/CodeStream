import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5500,
    proxy: {
      '/api/auth': {
        target: 'http://localhost:3000',
        rewrite: path => path.replace(/^\/api\/auth/, ''),
      },
      '/api/crdt': {
        target: 'http://localhost:3002',
        rewrite: path => path.replace(/^\/api\/crdt/, ''),
      },
      '/api/exec': {
        target: 'http://localhost:3003',
        rewrite: path => path.replace(/^\/api\/exec/, ''),
      },
      '/ws': {
        target: 'ws://localhost:3001',
        ws: true,
        rewrite: path => path,
      },
    },
  },
});
