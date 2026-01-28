import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Plus, Trash2, Copy, Key, Check } from 'lucide-react';
import { cn } from '../lib/utils';
import { apiKeysApi } from '../lib/api';

interface ApiKeyItem {
  id: string;
  name: string;
  key_prefix: string;
  permissions: string[];
  created_at: string;
}

const AVAILABLE_PERMISSIONS = [
  { value: '*', label: 'Full Access' },
  { value: 'routes:read', label: 'Routes (Read)' },
  { value: 'routes:write', label: 'Routes (Write)' },
  { value: 'users:read', label: 'Users (Read)' },
  { value: 'stats:read', label: 'Stats (Read)' },
];

export default function ApiKeys() {
  const { t } = useTranslation();
  const [apiKeys, setApiKeys] = useState<ApiKeyItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [newKeyName, setNewKeyName] = useState('');
  const [newKeyPermissions, setNewKeyPermissions] = useState<string[]>(['*']);
  const [showNewKey, setShowNewKey] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const loadKeys = useCallback(async () => {
    setLoading(true);
    const res = await apiKeysApi.list();
    if (res.data) {
      setApiKeys(res.data.api_keys);
    }
    setLoading(false);
  }, []);

  useEffect(() => { loadKeys(); }, [loadKeys]);

  const handleCreate = async () => {
    if (!newKeyName.trim()) return;
    setCreating(true);
    const res = await apiKeysApi.create(newKeyName.trim(), newKeyPermissions);
    if (res.data) {
      setShowNewKey(res.data.raw_key);
      setNewKeyName('');
      setNewKeyPermissions(['*']);
      await loadKeys();
    }
    setCreating(false);
  };

  const handleDelete = async (id: string) => {
    await apiKeysApi.delete(id);
    setApiKeys(apiKeys.filter(k => k.id !== id));
  };

  const handleCopy = async (key: string) => {
    await navigator.clipboard.writeText(key);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const togglePermission = (perm: string) => {
    if (perm === '*') {
      setNewKeyPermissions(['*']);
    } else {
      const filtered = newKeyPermissions.filter(p => p !== '*');
      if (filtered.includes(perm)) {
        setNewKeyPermissions(filtered.filter(p => p !== perm));
      } else {
        setNewKeyPermissions([...filtered, perm]);
      }
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-3xl font-bold tracking-tight">{t('apiKeys.title')}</h2>
      <p className="text-slate-500">{t('apiKeys.description')}</p>

      {showNewKey && (
        <Card className="border-green-200 bg-green-50">
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div className="space-y-1">
                <p className="text-sm font-medium text-green-800">{t('apiKeys.newKeyCreated')}</p>
                <code className="block text-xs bg-white px-3 py-2 rounded border font-mono break-all">
                  {showNewKey}
                </code>
                <p className="text-xs text-green-600">{t('apiKeys.copyWarning')}</p>
              </div>
              <Button
                size="sm"
                variant="outline"
                onClick={() => handleCopy(showNewKey)}
                className="ml-4 shrink-0"
              >
                {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
              </Button>
            </div>
            <Button
              size="sm"
              variant="ghost"
              className="mt-4"
              onClick={() => setShowNewKey(null)}
            >
              {t('apiKeys.dismiss')}
            </Button>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t('apiKeys.createNew')}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('apiKeys.name')}</label>
              <Input
                value={newKeyName}
                onChange={(e) => setNewKeyName(e.target.value)}
                placeholder={t('apiKeys.namePlaceholder')}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('apiKeys.permissions')}</label>
              <div className="flex flex-wrap gap-2">
                {AVAILABLE_PERMISSIONS.map((perm) => (
                  <button
                    key={perm.value}
                    onClick={() => togglePermission(perm.value)}
                    className={cn(
                      "px-3 py-1 text-xs rounded-full border transition-colors",
                      newKeyPermissions.includes(perm.value)
                        ? "bg-indigo-100 border-indigo-300 text-indigo-700"
                        : "bg-slate-50 border-slate-200 text-slate-600 hover:bg-slate-100"
                    )}
                  >
                    {perm.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
          <Button onClick={handleCreate} disabled={creating || !newKeyName.trim()}>
            <Plus className="w-4 h-4 mr-2" />
            {t('apiKeys.generate')}
          </Button>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('apiKeys.existingKeys')}</CardTitle>
        </CardHeader>
        <CardContent>
          {loading && <p className="text-slate-500">{t('common.loading')}</p>}
          {!loading && apiKeys.length === 0 && (
            <p className="text-slate-500 text-center py-8">{t('apiKeys.noKeys')}</p>
          )}
          {!loading && apiKeys.length > 0 && (
            <div className="space-y-3">
              {apiKeys.map((key) => (
                <div
                  key={key.id}
                  className="flex items-center justify-between p-4 border rounded-lg bg-slate-50/50"
                >
                  <div className="flex items-center gap-4">
                    <Key className="w-5 h-5 text-slate-400" />
                    <div>
                      <p className="font-medium">{key.name}</p>
                      <p className="text-xs text-slate-500">
                        {key.key_prefix}••••••••
                        <span className="mx-2">•</span>
                        {new Date(key.created_at).toLocaleDateString()}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="flex gap-1">
                      {(key.permissions as string[]).map((p) => (
                        <span
                          key={p}
                          className="px-2 py-0.5 text-xs bg-slate-200 rounded"
                        >
                          {p}
                        </span>
                      ))}
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-red-500 hover:text-red-600 hover:bg-red-50"
                      onClick={() => handleDelete(key.id)}
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
