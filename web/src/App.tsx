import { Navbar } from '@fliqqs/portfolio-scaffold'
import WasmDemo from './components/WasmDemo'
import './App.css'

function App() {

  return (
    <div className="app">
      <Navbar
        title="client-side-prediction"
        siteName="fliqqs"
        homeUrl="https://fliqqs.github.io"
        links={[]}
      />
      <main className="container">
        <div className="description">
          <p>
            A demonstration of client-side prediction, server reconciliation, and entity interpolation 
            for networked games. Built with Rust and WebAssembly.
          </p>
        </div>
        <WasmDemo />
      </main>
    </div>
  )
}

export default App
