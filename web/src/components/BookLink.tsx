import type { ReactNode } from 'react';
import type { Language } from '../i18n/resources';

interface BookLinkProps {
  base: string;
  language: Language;
  book: 'user-guide' | 'dev-guide';
  page?: string;
  className?: string;
  children: ReactNode;
}

export function BookLink({
  base,
  language,
  book,
  page = '',
  className,
  children,
}: BookLinkProps) {
  return (
    <a
      className={className}
      data-book={book}
      href={`${base}book/${book}/${language}/${page}`}
    >
      {children}
    </a>
  );
}
