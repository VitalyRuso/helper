import { Menu, Search } from "lucide-react";
import { useState } from "react";
import { Link, NavLink } from "react-router-dom";
import { ChatWidget } from "../chat/ChatWidget";

const links = [
  ["/guides", "Гайды"],
  ["/kb", "База знаний"],
  ["/assistant", "ИИ-помощник"],
  ["/document-analyzer", "Документы"],
  ["/pricing", "Доступ"],
];

export function AppLayout({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="min-h-screen bg-paper">
      <header className="sticky top-0 z-30 border-b border-ink/10 bg-paper/90 backdrop-blur">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-4 py-3">
          <Link to="/" className="flex items-center gap-3 font-semibold">
            <span className="grid h-9 w-9 place-items-center rounded-md bg-brand text-white">SH</span>
            <span>Spain Helper AI</span>
          </Link>
          <nav className="hidden items-center gap-6 text-sm md:flex">
            {links.map(([to, label]) => (
              <NavLink key={to} to={to} className={({ isActive }) => (isActive ? "text-brand" : "text-ink/70 hover:text-ink")}>
                {label}
              </NavLink>
            ))}
            <Link to="/search" className="rounded-md p-2 hover:bg-white" title="Поиск">
              <Search className="h-5 w-5" />
            </Link>
          </nav>
          <button className="rounded-md p-2 md:hidden" onClick={() => setOpen((v) => !v)} title="Меню">
            <Menu className="h-6 w-6" />
          </button>
        </div>
        {open && (
          <div className="border-t border-ink/10 bg-white px-4 py-3 md:hidden">
            {links.map(([to, label]) => (
              <Link key={to} to={to} className="block py-2" onClick={() => setOpen(false)}>
                {label}
              </Link>
            ))}
          </div>
        )}
      </header>
      <main>{children}</main>
      <footer className="border-t border-ink/10 px-4 py-8 text-sm text-ink/60">
        <div className="mx-auto flex max-w-7xl flex-col gap-2 md:flex-row md:justify-between">
          <span>Spain Helper AI — информационный сервис, не юридическая консультация.</span>
          <div className="flex gap-4">
            <Link to="/about">О проекте</Link>
            <Link to="/legal">Правовая информация</Link>
            <Link to="/admin">Админ</Link>
          </div>
        </div>
      </footer>
      <ChatWidget />
    </div>
  );
}
