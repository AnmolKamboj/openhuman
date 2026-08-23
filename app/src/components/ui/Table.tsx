import { type ComponentPropsWithoutRef } from 'react';

import { cn } from '../../lib/cn';

/**
 * Radix has no Table primitive, so these are thin semantic wrappers that give
 * the eleven hand-rolled tables in the app one shared look and — more
 * usefully — one place that gets horizontal overflow right. A wide table that
 * cannot scroll inside its own container is what makes a whole page scroll
 * sideways.
 */
export const Table = ({ className, ...rest }: ComponentPropsWithoutRef<'table'>) => (
  <div className="w-full overflow-x-auto">
    <table
      data-slot="table"
      className={cn('w-full caption-bottom border-collapse text-sm', className)}
      {...rest}
    />
  </div>
);

export const TableHeader = ({ className, ...rest }: ComponentPropsWithoutRef<'thead'>) => (
  <thead
    data-slot="table-header"
    className={cn('[&_tr]:border-b [&_tr]:border-line', className)}
    {...rest}
  />
);

export const TableBody = ({ className, ...rest }: ComponentPropsWithoutRef<'tbody'>) => (
  <tbody data-slot="table-body" className={cn('[&_tr:last-child]:border-0', className)} {...rest} />
);

export const TableRow = ({ className, ...rest }: ComponentPropsWithoutRef<'tr'>) => (
  <tr
    data-slot="table-row"
    className={cn(
      'border-b border-line-subtle transition-colors hover:bg-surface-hover',
      className
    )}
    {...rest}
  />
);

export const TableHead = ({ className, ...rest }: ComponentPropsWithoutRef<'th'>) => (
  <th
    data-slot="table-head"
    scope="col"
    className={cn(
      'h-9 px-3 text-left align-middle text-xs font-medium text-content-muted',
      className
    )}
    {...rest}
  />
);

export const TableCell = ({ className, ...rest }: ComponentPropsWithoutRef<'td'>) => (
  <td
    data-slot="table-cell"
    className={cn('px-3 py-2 align-middle text-content', className)}
    {...rest}
  />
);

export default Table;
