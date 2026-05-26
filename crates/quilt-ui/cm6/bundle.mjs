// esbuild bundler for the CM6 editor bridge.
// Produces a single JS file that sets window.__quiltCm6.
import * as esbuild from 'esbuild';

const isWatch = process.argv.includes('--watch');

/** @type {esbuild.BuildOptions} */
const config = {
  entryPoints: ['src/index.js'],
  outfile: 'dist/quilt-cm6.js',
  bundle: true,
  format: 'iife',
  globalName: '__quiltCm6Bundle',
  // We expose the API via window.__quiltCm6 inside the bundle
  // so the globalName is just internal scaffolding.
  minify: false,
  sourcemap: false,
  target: 'es2020',
  logLevel: 'info',
};

async function main() {
  if (isWatch) {
    const ctx = await esbuild.context(config);
    await ctx.watch();
    console.log('watching for changes...');
  } else {
    await esbuild.build(config);
    console.log('built dist/quilt-cm6.js');
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
