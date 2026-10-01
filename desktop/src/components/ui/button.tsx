import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function Button({ className, variant = 'primary', type = 'button', ...props }:
  ComponentProps<'button'> & { variant?: 'primary' | 'outline' }) {
  return <button type={type} className={cn(
    'inline-flex items-center justify-center gap-2 rounded-lg border px-4 py-2.5 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring disabled:opacity-40',
    variant === 'primary' ? 'border-blue-400/30 bg-blue-500 text-white hover:bg-blue-400' : 'border-border bg-transparent text-slate-300 hover:bg-muted',
    className,
  )} {...props} />;
}
