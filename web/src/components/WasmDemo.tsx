import { useEffect, useRef } from 'react'
import './WasmDemo.css'

declare global {
  interface Window {
    load: (wasmPath: string) => void;
  }
}

const WasmDemo = () => {
  const wasmLoadedRef = useRef(false)

  useEffect(() => {
    if (wasmLoadedRef.current) return
    wasmLoadedRef.current = true

    const loadWasm = async () => {
      try {
        // Load miniquad JS bundle
        const miniquadScript = document.createElement('script')
        miniquadScript.src = 'https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js'
        miniquadScript.async = true
        
        miniquadScript.onload = () => {
          console.log('Miniquad bundle loaded')
          // Load the WASM module after miniquad is ready
          if (window.load) {
            window.load('netcode_example.wasm')
            console.log('WASM module loaded successfully')
          }
        }
        
        miniquadScript.onerror = () => {
          console.error('Failed to load miniquad bundle')
        }
        
        document.body.appendChild(miniquadScript)

        return () => {
          document.body.removeChild(miniquadScript)
        }
      } catch (error) {
        console.error('Error loading WASM:', error)
      }
    }

    loadWasm()
  }, [])

  return (
    <div className="wasm-demo-container">
      <canvas 
        id="glcanvas"
        tabIndex={0}
      />
    </div>
  )
}

export default WasmDemo
