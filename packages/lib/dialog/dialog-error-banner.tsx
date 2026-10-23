import { AlertTriangle } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { fadeDownVariants, transitionBase } from '../motion/variants';

export interface DialogErrorBannerProps {
  message?: string | null;
}

export function DialogErrorBanner({ message }: DialogErrorBannerProps) {
  return (
    <AnimatePresence>
      {message && (
        <motion.div
          variants={fadeDownVariants}
          initial="initial"
          animate="animate"
          exit="exit"
          transition={transitionBase}
          role="alert"
          className="bg-status-cancelled text-status-cancelled-fg flex items-start gap-2 rounded-xl px-4 py-3 text-sm"
        >
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <span>{message}</span>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
