import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5174,
    strictPort: false,
    proxy: {
      '/api': {
        target: 'http://localhost:9090',
        ws: true,
      },
      '/ws': {
        target: 'ws://localhost:9090',
        ws: true,
      },
      // Local `npm run mock:prometheus` for the saturation heatmap.
      '/mock-prom': {
        target: 'http://localhost:9091',
        rewrite: (path) => path.replace(/^\/mock-prom/, ''),
      },
    },
  },
});
