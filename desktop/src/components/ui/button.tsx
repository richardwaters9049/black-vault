import type { ComponentProps } from 'react';
import { cn } from '@/lib/utils';

export function Button({ className, variant = 'primary', type = 'button', ...props }:
  ComponentProps<'button'> & { variant?: 'primary' | 'outline' }) {
  return <button type={type} className={cn(
    'inline-flex min-h-11 items-center justify-center gap-2 rounded-xl border px-4 py-2.5 text-sm font-medium transition-[background-color,border-color,color,box-shadow] focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring disabled:opacity-40',
    variant === 'primary'
      ? 'border-blue-400/30 bg-blue-500 text-white shadow-[0_8px_24px_#2563eb24] hover:border-blue-300/40 hover:bg-blue-400 hover:shadow-[0_10px_28px_#2563eb32]'
      : 'border-border bg-white/[0.015] text-slate-300 hover:border-blue-400/20 hover:bg-muted hover:text-white',
    className,
  )} {...props} />;
}
