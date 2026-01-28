import { Link, useLocation, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { LayoutDashboard, Server, Users, LogOut, FlaskConical, KeyRound } from 'lucide-react';
import { cn } from '../lib/utils';
import { Button } from './ui/Button';
import { LanguageToggle } from './LanguageToggle';
import { useAuth } from '../context/AuthContext';

export function Layout() {
  const { t } = useTranslation();
  const location = useLocation();
  const { logout } = useAuth();

  const navItems = [
    { href: '/', icon: LayoutDashboard, label: t('common.dashboard') },
    { href: '/proxy', icon: Server, label: t('common.proxyConfig') },
    { href: '/users', icon: Users, label: t('common.users') },
    { href: '/api-keys', icon: KeyRound, label: t('common.apiKeys') },
    { href: '/auth-test', icon: FlaskConical, label: 'Auth Test' },
  ];

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex">
      <aside className="w-64 bg-slate-900 text-white flex flex-col fixed h-full z-10">
        <div className="p-6">
          <h1 className="text-xl font-bold tracking-wider">ARC AUTH</h1>
          <p className="text-xs text-slate-400 mt-1">Admin Console</p>
        </div>
        
        <nav className="flex-1 px-4 space-y-2 mt-4">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname === item.href;
            return (
              <Link
                key={item.href}
                to={item.href}
                className={cn(
                  "flex items-center gap-3 px-4 py-3 rounded-lg text-sm font-medium transition-colors",
                  isActive 
                    ? "bg-indigo-600 text-white shadow-lg shadow-indigo-900/20" 
                    : "text-slate-400 hover:text-white hover:bg-slate-800"
                )}
              >
                <Icon className="w-5 h-5" />
                {item.label}
              </Link>
            );
          })}
        </nav>

        <div className="p-4 border-t border-slate-800">
          <div className="flex items-center justify-between px-4 py-2">
            <LanguageToggle />
            <Button 
              variant="ghost" 
              size="icon" 
              onClick={logout}
              className="text-slate-400 hover:text-white hover:bg-slate-800"
            >
              <LogOut className="w-5 h-5" />
            </Button>
          </div>
        </div>
      </aside>

      <main className="flex-1 ml-64 p-8">
        <div className="max-w-7xl mx-auto">
          <Outlet />
        </div>
      </main>
    </div>
  );
}
