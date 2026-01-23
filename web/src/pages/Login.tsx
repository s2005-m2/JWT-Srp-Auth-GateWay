import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../context/AuthContext';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { ShieldCheck } from 'lucide-react';
import { adminApi } from '../lib/api';

type AuthMode = 'login' | 'register';

export default function Login() {
  const { t } = useTranslation();
  const { login } = useAuth();
  const navigate = useNavigate();
  const [mode, setMode] = useState<AuthMode>('login');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [registrationToken, setRegistrationToken] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);
    
    try {
      const result = mode === 'login' 
        ? await adminApi.login(username, password)
        : await adminApi.register(username, password, registrationToken);
      
      if (result.error) {
        throw new Error(result.error.message);
      }
      
      if (result.data) {
        login(result.data.access_token);
        navigate('/');
      }
    } catch (err: any) {
      setError(err.message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 px-4">
      <Card className="w-full max-w-md">
        <CardHeader className="space-y-1 flex flex-col items-center">
          <div className="w-12 h-12 bg-indigo-600 rounded-lg flex items-center justify-center mb-4">
            <ShieldCheck className="w-8 h-8 text-white" />
          </div>
          <CardTitle className="text-2xl font-bold text-center">
            {t('login.title')}
          </CardTitle>
          <div className="flex gap-2 mt-2">
            <button
              type="button"
              onClick={() => setMode('login')}
              className={`px-3 py-1 text-sm rounded ${mode === 'login' ? 'bg-indigo-600 text-white' : 'bg-gray-200 text-gray-700'}`}
            >
              {t('common.login')}
            </button>
            <button
              type="button"
              onClick={() => setMode('register')}
              className={`px-3 py-1 text-sm rounded ${mode === 'register' ? 'bg-indigo-600 text-white' : 'bg-gray-200 text-gray-700'}`}
            >
              {t('login.register')}
            </button>
          </div>
        </CardHeader>
        <CardContent>
          {error && (
            <div className="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded text-sm">
              {error}
            </div>
          )}
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('login.username')}</label>
              <Input
                type="text"
                placeholder="admin"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                required
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('login.password')}</label>
              <Input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
              />
            </div>
            {mode === 'register' && (
              <div className="space-y-2">
                <label className="text-sm font-medium">{t('login.registrationToken')}</label>
                <Input
                  type="text"
                  placeholder={t('login.tokenPlaceholder')}
                  value={registrationToken}
                  onChange={(e) => setRegistrationToken(e.target.value)}
                  required
                />
              </div>
            )}
            <Button className="w-full bg-indigo-600 hover:bg-indigo-700" type="submit" isLoading={isLoading}>
              {mode === 'login' ? t('login.submit') : t('login.registerSubmit')}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
