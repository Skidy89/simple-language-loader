export default {
  timeout: '2m',
  serial: true,
  verbose: true,
  extensions: {
    ts: 'module'
  },
  nodeArguments: [
    '--import=tsx'
  ],
  workerThreads: false
}
