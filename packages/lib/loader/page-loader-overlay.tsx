import { motion } from 'framer-motion';
import { AppLogo } from '../../components/app-logo';
import { loadingDotVariants, loadingDotTransition } from '../motion/variants';

/*
 * Three dots pulsing in sequence, not a spinning ring - reads calmer and more deliberate for a cover that can sit on screen for a beat.
 * Each dot's `delay` is offset from the last so the pulse travels across the row rather than firing in unison.
 */
function LoadingEllipsis() {
  return (
    <div className="flex items-center gap-1.5" aria-hidden="true">
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className="h-2 w-2 rounded-full bg-white"
          variants={loadingDotVariants}
          animate="animate"
          transition={{ ...loadingDotTransition, delay: i * 0.2 }}
        />
      ))}
    </div>
  );
}

/*
 * The app-wide loading cover: an auth bootstrap, a logout, any transition with no page underneath it yet to show its own skeleton.
 * Sits one level above the dialog engine's own blocking overlay (`DialogLoaderOverlay`, z-90), so it can cover a dialog that triggered it too.
 * Reuses AppLogo rather than an inline image path, so a brand refresh is one file, not a grep for every hardcoded logo src.
 */
export function PageLoaderOverlay() {
  return (
    <div
      className="fixed inset-0 z-100 flex flex-col items-center justify-center gap-4 bg-black/30 backdrop-blur-sm"
      role="status"
      aria-live="polite"
    >
      <AppLogo variant="white" />
      <LoadingEllipsis />
      <span className="sr-only">Loading</span>
    </div>
  );
}
