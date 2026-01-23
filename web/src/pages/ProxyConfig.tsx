import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Plus, Trash2, Save, Route, Shield, Timer } from 'lucide-react';
import { cn } from '../lib/utils';
import { configApi } from '../lib/api';

type ConfigTab = 'routes' | 'jwt' | 'ratelimits';

interface RouteItem {
  id: string;
  path_prefix: string;
  upstream_address: string;
  require_auth: boolean;
  enabled: boolean;
}

interface RateLimitItem {
  id: string;
  name: string;
  path_pattern: string;
  max_requests: number;
  window_secs: number;
  enabled: boolean;
}

interface JwtConfig {
  access_token_ttl_secs: number;
  refresh_token_ttl_secs: number;
  auto_refresh_threshold_secs: number;
}

export default function ProxyConfig() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<ConfigTab>('routes');
  const [routes, setRoutes] = useState<RouteItem[]>([]);
  const [rateLimits, setRateLimits] = useState<RateLimitItem[]>([]);
  const [jwtConfig, setJwtConfig] = useState<JwtConfig | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setLoading(true);
    const [routesRes, limitsRes, jwtRes] = await Promise.all([
      configApi.listRoutes(),
      configApi.listRateLimits(),
      configApi.getJwtConfig(),
    ]);
    if (routesRes.data) setRoutes(routesRes.data);
    if (limitsRes.data) setRateLimits(limitsRes.data);
    if (jwtRes.data) setJwtConfig(jwtRes.data);
    setLoading(false);
  };

  const handleSaveJwt = async () => {
    if (!jwtConfig) return;
    await configApi.updateJwtConfig(jwtConfig);
  };

  const handleDeleteRoute = async (id: string) => {
    await configApi.deleteRoute(id);
    setRoutes(routes.filter(r => r.id !== id));
  };

  const handleDeleteRateLimit = async (id: string) => {
    await configApi.deleteRateLimit(id);
    setRateLimits(rateLimits.filter(r => r.id !== id));
  };

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
                {loading && <p className="text-slate-500">Loading...</p>}
                {routes.map((route) => (
                  <div key={route.id} className="flex items-start space-x-4 p-4 border rounded-lg bg-slate-50/50">
                    <div className="flex-1 grid grid-cols-3 gap-4">
                      <div className="space-y-2">
                        <label className="text-sm font-medium">{t('proxy.pathPrefix')}</label>
                        <Input defaultValue={route.path_prefix} />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">{t('proxy.upstreamAddress')}</label>
                        <Input defaultValue={route.upstream_address} />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">{t('proxy.requireAuth')}</label>
                        <select 
                          className="flex h-10 w-full rounded-md border border-slate-200 bg-white px-3 py-2 text-sm"
                          defaultValue={route.require_auth ? 'true' : 'false'}
                        >
                          <option value="true">{t('common.yes')}</option>
                          <option value="false">{t('common.no')}</option>
                        </select>
                      </div>
                    </div>
                    <Button 
                      variant="ghost" 
                      size="icon" 
                      className="text-red-500 hover:text-red-600 hover:bg-red-50"
                      onClick={() => handleDeleteRoute(route.id)}
                    >
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
              <div className="grid gap-6 md:grid-cols-3">
                <div className="space-y-2">
                  <label className="text-sm font-medium">Access Token TTL (sec)</label>
                  <Input 
                    type="number" 
                    value={jwtConfig?.access_token_ttl_secs ?? ''} 
                    onChange={(e) => setJwtConfig(prev => prev ? {...prev, access_token_ttl_secs: Number(e.target.value)} : null)}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Refresh Token TTL (sec)</label>
                  <Input 
                    type="number" 
                    value={jwtConfig?.refresh_token_ttl_secs ?? ''} 
                    onChange={(e) => setJwtConfig(prev => prev ? {...prev, refresh_token_ttl_secs: Number(e.target.value)} : null)}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Auto Refresh Threshold (sec)</label>
                  <Input 
                    type="number" 
                    value={jwtConfig?.auto_refresh_threshold_secs ?? ''} 
                    onChange={(e) => setJwtConfig(prev => prev ? {...prev, auto_refresh_threshold_secs: Number(e.target.value)} : null)}
                  />
                </div>
              </div>
              <Button onClick={handleSaveJwt}>
                <Save className="w-4 h-4 mr-2" />
                {t('common.save')}
              </Button>
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
                {loading && <p className="text-slate-500">Loading...</p>}
                {rateLimits.map((item) => (
                  <div key={item.id} className="flex items-start space-x-4 p-4 border rounded-lg bg-slate-50/50">
                    <div className="flex-1 grid grid-cols-4 gap-4">
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Name</label>
                        <Input defaultValue={item.name} />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Path Pattern</label>
                        <Input defaultValue={item.path_pattern} />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Limit (req)</label>
                        <Input type="number" defaultValue={item.max_requests} />
                      </div>
                      <div className="space-y-2">
                        <label className="text-sm font-medium">Window (sec)</label>
                        <Input type="number" defaultValue={item.window_secs} />
                      </div>
                    </div>
                    <Button 
                      variant="ghost" 
                      size="icon" 
                      className="text-red-500 hover:text-red-600 hover:bg-red-50"
                      onClick={() => handleDeleteRateLimit(item.id)}
                    >
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
