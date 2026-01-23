import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Plus, Trash2, Save, Route, Shield, Timer } from 'lucide-react';
import { cn } from '../lib/utils';

type ConfigTab = 'routes' | 'jwt' | 'ratelimits';

export default function ProxyConfig() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<ConfigTab>('routes');

  const tabs = [
    { id: 'routes', label: t('proxy.routes'), icon: Route },
    { id: 'jwt', label: t('proxy.jwtSettings'), icon: Shield },
    { id: 'ratelimits', label: t('proxy.rateLimits'), icon: Timer },
  ];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold tracking-tight">{t('proxy.title')}</h2>
        <Button>
          <Save className="w-4 h-4 mr-2" />
          {t('common.save')}
        </Button>
      </div>

      <div className="flex space-x-1 bg-slate-100 p-1 rounded-lg w-fit">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as ConfigTab)}
              className={cn(
                "flex items-center px-4 py-2 text-sm font-medium rounded-md transition-all",
                activeTab === tab.id
                  ? "bg-white text-slate-900 shadow-sm"
                  : "text-slate-500 hover:text-slate-900 hover:bg-slate-200/50"
              )}
            >
              <Icon className="w-4 h-4 mr-2" />
              {tab.label}
            </button>
          );
        })}
      </div>

      <div className="space-y-6">
        {activeTab === 'routes' && (
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>{t('proxy.routes')}</CardTitle>
              <Button size="sm" variant="outline">
                <Plus className="w-4 h-4 mr-2" />
                {t('proxy.addRoute')}
              </Button>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {[1, 2].map((i) => (
                  <div key={i} className="flex items-start space-x-4 p-4 border rounded-lg bg-slate-50/50">
                    <div className="flex-1 grid grid-cols-3 gap-4">
                      <div className="space-y-2">
                        <label className="text-sm font-medium">{t('proxy.pathPrefix')}</label>
                        <Input defaultValue={i === 1 ? "/api/" : "/ws/"} />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">{t('proxy.upstreamAddress')}</label>
                        <Input defaultValue="127.0.0.1:7000" />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">{t('proxy.requireAuth')}</label>
                        <select className="flex h-10 w-full rounded-md border border-slate-200 bg-white px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950">
                          <option value="true">{t('common.yes')}</option>
                          <option value="false">{t('common.no')}</option>
                        </select>
                      </div>
                    </div>
                    <Button variant="ghost" size="icon" className="text-red-500 hover:text-red-600 hover:bg-red-50">
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}

        {activeTab === 'jwt' && (
          <Card>
            <CardHeader>
              <CardTitle>{t('proxy.jwtSettings')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid gap-6 md:grid-cols-2">
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.jwtSecret')}</label>
                  <Input type="password" value="************************" readOnly />
                  <p className="text-xs text-slate-500">Secret key used for signing tokens</p>
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.tokenTTL')}</label>
                  <Input type="number" defaultValue="3600" />
                  <p className="text-xs text-slate-500">Token validity duration in seconds</p>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {activeTab === 'ratelimits' && (
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <CardTitle>{t('proxy.rateLimits')}</CardTitle>
              <Button size="sm" variant="outline">
                <Plus className="w-4 h-4 mr-2" />
                {t('common.add')}
              </Button>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {[1].map((i) => (
                  <div key={i} className="flex items-start space-x-4 p-4 border rounded-lg bg-slate-50/50">
                    <div className="flex-1 grid grid-cols-3 gap-4">
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Endpoint Pattern</label>
                        <Input defaultValue="/api/*" />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Limit (req)</label>
                        <Input type="number" defaultValue="100" />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Window (sec)</label>
                        <Input type="number" defaultValue="60" />
                      </div>
                    </div>
                    <Button variant="ghost" size="icon" className="text-red-500 hover:text-red-600 hover:bg-red-50">
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
