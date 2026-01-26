import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Plus, Trash2, Save, Route, Shield, Timer, Mail, Check } from 'lucide-react';
import { cn } from '../lib/utils';
import { configApi } from '../lib/api';

type ConfigTab = 'routes' | 'jwt' | 'ratelimits' | 'smtp';

interface RouteItem {
  id: string;
  path_prefix: string;
  upstream_address: string;
  require_auth: boolean;
  strip_prefix: string | null;
  enabled: boolean;
  isNew?: boolean;
  isEditing?: boolean;
}

interface RateLimitItem {
  id: string;
  name: string;
  path_pattern: string;
  limit_by: string;
  max_requests: number;
  window_secs: number;
  enabled: boolean;
  isNew?: boolean;
  isEditing?: boolean;
}

interface JwtConfig {
  access_token_ttl_secs: number;
  refresh_token_ttl_secs: number;
  auto_refresh_threshold_secs: number;
}

interface SmtpConfig {
  from_email: string;
  smtp_pass: string;
}

export default function ProxyConfig() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<ConfigTab>('routes');
  const [routes, setRoutes] = useState<RouteItem[]>([]);
  const [rateLimits, setRateLimits] = useState<RateLimitItem[]>([]);
  const [jwtConfig, setJwtConfig] = useState<JwtConfig | null>(null);
  const [smtpConfig, setSmtpConfig] = useState<SmtpConfig>({
    from_email: '', smtp_pass: ''
  });
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  const loadData = useCallback(async () => {
    setLoading(true);
    const [routesRes, limitsRes, jwtRes, smtpRes] = await Promise.all([
      configApi.listRoutes(),
      configApi.listRateLimits(),
      configApi.getJwtConfig(),
      configApi.getSmtpConfig(),
    ]);
    if (routesRes.data) setRoutes(routesRes.data.map(r => ({ ...r, isEditing: false })));
    if (limitsRes.data) setRateLimits(limitsRes.data.map(r => ({ ...r, limit_by: r.limit_by || 'ip', isEditing: false })));
    if (jwtRes.data) setJwtConfig(jwtRes.data);
    if (smtpRes.data) setSmtpConfig({ from_email: smtpRes.data.from_email, smtp_pass: '' });
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const tabs = [
    { id: 'routes', label: t('proxy.routes'), icon: Route },
    { id: 'jwt', label: t('proxy.jwtSettings'), icon: Shield },
    { id: 'ratelimits', label: t('proxy.rateLimits'), icon: Timer },
    { id: 'smtp', label: t('proxy.smtp'), icon: Mail },
  ];

  const handleAddRoute = () => {
    const newRoute: RouteItem = {
      id: `new-${Date.now()}`,
      path_prefix: '/new/',
      upstream_address: '127.0.0.1:8000',
      require_auth: true,
      strip_prefix: null,
      enabled: true,
      isNew: true,
      isEditing: true,
    };
    setRoutes([newRoute, ...routes]);
  };

  const handleSaveRoute = async (route: RouteItem) => {
    setSaving(true);
    if (route.isNew) {
      const res = await configApi.createRoute({
        path_prefix: route.path_prefix,
        upstream_address: route.upstream_address,
        require_auth: route.require_auth,
        strip_prefix: route.strip_prefix || undefined,
      });
      if (res.data) {
        const newData: RouteItem = { ...res.data, isEditing: false };
        setRoutes(routes.map(r => r.id === route.id ? newData : r));
      }
    } else {
      const res = await configApi.updateRoute(route.id, {
        path_prefix: route.path_prefix,
        upstream_address: route.upstream_address,
        require_auth: route.require_auth,
        strip_prefix: route.strip_prefix || undefined,
        enabled: route.enabled,
      });
      if (res.data) {
        const newData: RouteItem = { ...res.data, isEditing: false };
        setRoutes(routes.map(r => r.id === route.id ? newData : r));
      }
    }
    setSaving(false);
  };

  const handleDeleteRoute = async (id: string, isNew?: boolean) => {
    if (isNew) {
      setRoutes(routes.filter(r => r.id !== id));
      return;
    }
    await configApi.deleteRoute(id);
    setRoutes(routes.filter(r => r.id !== id));
  };

  const handleAddRateLimit = () => {
    const newLimit: RateLimitItem = {
      id: `new-${Date.now()}`,
      name: 'New Rule',
      path_pattern: '/api/*',
      limit_by: 'ip',
      max_requests: 100,
      window_secs: 60,
      enabled: true,
      isNew: true,
      isEditing: true,
    };
    setRateLimits([newLimit, ...rateLimits]);
  };

  const handleSaveRateLimit = async (item: RateLimitItem) => {
    setSaving(true);
    if (item.isNew) {
      const res = await configApi.createRateLimit({
        name: item.name,
        path_pattern: item.path_pattern,
        limit_by: item.limit_by,
        max_requests: item.max_requests,
        window_secs: item.window_secs,
      });
      if (res.data) {
        const newData: RateLimitItem = { ...res.data, isEditing: false };
        setRateLimits(rateLimits.map(r => r.id === item.id ? newData : r));
      }
    } else {
      const res = await configApi.updateRateLimit(item.id, {
        name: item.name,
        path_pattern: item.path_pattern,
        limit_by: item.limit_by,
        max_requests: item.max_requests,
        window_secs: item.window_secs,
        enabled: item.enabled,
      });
      if (res.data) {
        const newData: RateLimitItem = { ...res.data, isEditing: false };
        setRateLimits(rateLimits.map(r => r.id === item.id ? newData : r));
      }
    }
    setSaving(false);
  };

  const handleDeleteRateLimit = async (id: string, isNew?: boolean) => {
    if (isNew) {
      setRateLimits(rateLimits.filter(r => r.id !== id));
      return;
    }
    await configApi.deleteRateLimit(id);
    setRateLimits(rateLimits.filter(r => r.id !== id));
  };

  const handleSaveJwt = async () => {
    if (!jwtConfig) return;
    setSaving(true);
    await configApi.updateJwtConfig(jwtConfig);
    setSaving(false);
  };

  const handleSaveSmtp = async () => {
    setSaving(true);
    await configApi.updateSmtpConfig(smtpConfig);
    setSaving(false);
  };

  const updateRoute = (id: string, updates: Partial<RouteItem>) => {
    setRoutes(routes.map(r => r.id === id ? { ...r, ...updates } : r));
  };

  const updateRateLimit = (id: string, updates: Partial<RateLimitItem>) => {
    setRateLimits(rateLimits.map(r => r.id === id ? { ...r, ...updates } : r));
  };

  return (
    <div className="space-y-6">
      <h2 className="text-3xl font-bold tracking-tight">{t('proxy.title')}</h2>

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

      {loading && <p className="text-slate-500">{t('common.loading')}</p>}

      {!loading && activeTab === 'routes' && (
        <RoutesTab
          routes={routes}
          onAdd={handleAddRoute}
          onSave={handleSaveRoute}
          onDelete={handleDeleteRoute}
          onUpdate={updateRoute}
          saving={saving}
          t={t}
        />
      )}

      {!loading && activeTab === 'jwt' && jwtConfig && (
        <JwtTab config={jwtConfig} onChange={setJwtConfig} onSave={handleSaveJwt} saving={saving} t={t} />
      )}

      {!loading && activeTab === 'ratelimits' && (
        <RateLimitsTab
          items={rateLimits}
          onAdd={handleAddRateLimit}
          onSave={handleSaveRateLimit}
          onDelete={handleDeleteRateLimit}
          onUpdate={updateRateLimit}
          saving={saving}
          t={t}
        />
      )}

      {!loading && activeTab === 'smtp' && (
        <SmtpTab config={smtpConfig} onChange={setSmtpConfig} onSave={handleSaveSmtp} saving={saving} t={t} />
      )}
    </div>
  );
}

interface RoutesTabProps {
  routes: RouteItem[];
  onAdd: () => void;
  onSave: (route: RouteItem) => Promise<void>;
  onDelete: (id: string, isNew?: boolean) => Promise<void>;
  onUpdate: (id: string, updates: Partial<RouteItem>) => void;
  saving: boolean;
  t: (key: string) => string;
}

function RoutesTab({ routes, onAdd, onSave, onDelete, onUpdate, saving, t }: RoutesTabProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t('proxy.routes')}</CardTitle>
        <Button size="sm" variant="outline" onClick={onAdd}>
          <Plus className="w-4 h-4 mr-2" />
          {t('proxy.addRoute')}
        </Button>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {routes.length === 0 && <p className="text-slate-500 text-center py-4">No routes configured</p>}
          {routes.map((route) => (
            <div key={route.id} className={cn(
              "flex items-start space-x-4 p-4 border rounded-lg",
              route.isNew ? "bg-blue-50/50 border-blue-200" : "bg-slate-50/50"
            )}>
              <div className="flex-1 grid grid-cols-4 gap-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.pathPrefix')}</label>
                  <Input
                    value={route.path_prefix}
                    onChange={(e) => onUpdate(route.id, { path_prefix: e.target.value })}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.upstreamAddress')}</label>
                  <Input
                    value={route.upstream_address}
                    onChange={(e) => onUpdate(route.id, { upstream_address: e.target.value })}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.stripPrefix')}</label>
                  <Input
                    value={route.strip_prefix || ''}
                    placeholder={t('proxy.stripPrefixHint')}
                    onChange={(e) => onUpdate(route.id, { strip_prefix: e.target.value || null })}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.requireAuth')}</label>
                  <select
                    className="flex h-10 w-full rounded-md border border-slate-200 bg-white px-3 py-2 text-sm"
                    value={route.require_auth ? 'true' : 'false'}
                    onChange={(e) => onUpdate(route.id, { require_auth: e.target.value === 'true' })}
                  >
                    <option value="true">{t('common.yes')}</option>
                    <option value="false">{t('common.no')}</option>
                  </select>
                </div>
              </div>
              <div className="flex flex-col space-y-2">
                <Button
                  size="icon"
                  variant="ghost"
                  className="text-green-600 hover:text-green-700 hover:bg-green-50"
                  onClick={() => onSave(route)}
                  disabled={saving}
                >
                  <Check className="w-4 h-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="text-red-500 hover:text-red-600 hover:bg-red-50"
                  onClick={() => onDelete(route.id, route.isNew)}
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

interface JwtTabProps {
  config: JwtConfig;
  onChange: (config: JwtConfig) => void;
  onSave: () => Promise<void>;
  saving: boolean;
  t: (key: string) => string;
}

function JwtTab({ config, onChange, onSave, saving, t }: JwtTabProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('proxy.jwtSettings')}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid gap-6 md:grid-cols-3">
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('proxy.accessTokenTTL')}</label>
            <Input
              type="number"
              value={config.access_token_ttl_secs}
              onChange={(e) => onChange({ ...config, access_token_ttl_secs: Number(e.target.value) })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('proxy.refreshTokenTTL')}</label>
            <Input
              type="number"
              value={config.refresh_token_ttl_secs}
              onChange={(e) => onChange({ ...config, refresh_token_ttl_secs: Number(e.target.value) })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('proxy.autoRefreshThreshold')}</label>
            <Input
              type="number"
              value={config.auto_refresh_threshold_secs}
              onChange={(e) => onChange({ ...config, auto_refresh_threshold_secs: Number(e.target.value) })}
            />
          </div>
        </div>
        <Button onClick={onSave} disabled={saving}>
          <Save className="w-4 h-4 mr-2" />
          {t('common.save')}
        </Button>
      </CardContent>
    </Card>
  );
}

interface RateLimitsTabProps {
  items: RateLimitItem[];
  onAdd: () => void;
  onSave: (item: RateLimitItem) => Promise<void>;
  onDelete: (id: string, isNew?: boolean) => Promise<void>;
  onUpdate: (id: string, updates: Partial<RateLimitItem>) => void;
  saving: boolean;
  t: (key: string) => string;
}

function RateLimitsTab({ items, onAdd, onSave, onDelete, onUpdate, saving, t }: RateLimitsTabProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t('proxy.rateLimits')}</CardTitle>
        <Button size="sm" variant="outline" onClick={onAdd}>
          <Plus className="w-4 h-4 mr-2" />
          {t('common.add')}
        </Button>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {items.length === 0 && <p className="text-slate-500 text-center py-4">No rate limits configured</p>}
          {items.map((item) => (
            <div key={item.id} className={cn(
              "flex items-start space-x-4 p-4 border rounded-lg",
              item.isNew ? "bg-blue-50/50 border-blue-200" : "bg-slate-50/50"
            )}>
              <div className="flex-1 grid grid-cols-4 gap-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.name')}</label>
                  <Input
                    value={item.name}
                    onChange={(e) => onUpdate(item.id, { name: e.target.value })}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.pathPattern')}</label>
                  <Input
                    value={item.path_pattern}
                    onChange={(e) => onUpdate(item.id, { path_pattern: e.target.value })}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.maxRequests')}</label>
                  <Input
                    type="number"
                    value={item.max_requests}
                    onChange={(e) => onUpdate(item.id, { max_requests: Number(e.target.value) })}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">{t('proxy.windowSecs')}</label>
                  <Input
                    type="number"
                    value={item.window_secs}
                    onChange={(e) => onUpdate(item.id, { window_secs: Number(e.target.value) })}
                  />
                </div>
              </div>
              <div className="flex flex-col space-y-2">
                <Button
                  size="icon"
                  variant="ghost"
                  className="text-green-600 hover:text-green-700 hover:bg-green-50"
                  onClick={() => onSave(item)}
                  disabled={saving}
                >
                  <Check className="w-4 h-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="text-red-500 hover:text-red-600 hover:bg-red-50"
                  onClick={() => onDelete(item.id, item.isNew)}
                >
                  <Trash2 className="w-4 h-4" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

interface SmtpTabProps {
  config: SmtpConfig;
  onChange: (config: SmtpConfig) => void;
  onSave: () => Promise<void>;
  saving: boolean;
  t: (key: string) => string;
}

function SmtpTab({ config, onChange, onSave, saving, t }: SmtpTabProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('proxy.smtp')}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="grid gap-6 md:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('proxy.fromEmail')}</label>
            <Input
              value={config.from_email}
              placeholder="noreply@example.com"
              onChange={(e) => onChange({ ...config, from_email: e.target.value })}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">{t('proxy.smtpPass')}</label>
            <Input
              type="password"
              value={config.smtp_pass}
              placeholder="••••••••"
              onChange={(e) => onChange({ ...config, smtp_pass: e.target.value })}
            />
          </div>
        </div>
        <Button onClick={onSave} disabled={saving}>
          <Save className="w-4 h-4 mr-2" />
          {t('common.save')}
        </Button>
      </CardContent>
    </Card>
  );
}
