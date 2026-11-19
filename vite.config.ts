import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    // cargo 会写 src-tauri/target，编辑器的原子保存会先建 .<name>.tmpdir/ 临时目录；
    // Vite 监视到这两类被占用的文件会以 EBUSY 崩溃并连带终止 beforeDevCommand。
    watch: { ignored: ['**/src-tauri/**', '**/.*.tmpdir/**'] },
  },
})
