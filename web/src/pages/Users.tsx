import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Search, MoreHorizontal, UserPlus } from 'lucide-react';
import { useAuth } from '../context/AuthContext';

interface User {
  id: string;
  email: string;
  status: string;
  created_at: string;
  last_login: string | null;
}

export default function Users() {
  const { t } = useTranslation();
  const { token } = useAuth();
  const [users, setUsers] = useState<User[]>([]);
  const [search, setSearch] = useState('');

  useEffect(() => {
    const fetchUsers = async () => {
      const res = await fetch('/api/admin/users', {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        const data = await res.json();
        setUsers(data.users || []);
      }
    };
    fetchUsers();
  }, [token]);

  const filteredUsers = users.filter(u => 
    u.email.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold tracking-tight">{t('common.users')}</h2>
        <Button>
          <UserPlus className="w-4 h-4 mr-2" />
          {t('common.add')}
        </Button>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>{t('common.users')}</CardTitle>
            <div className="relative w-64">
              <Search className="absolute left-2 top-2.5 h-4 w-4 text-slate-500" />
              <Input 
                placeholder="Search users..." 
                className="pl-8" 
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="rounded-md border">
            <table className="w-full text-sm text-left">
              <thead className="bg-slate-50 text-slate-500 font-medium">
                <tr>
                  <th className="px-4 py-3">Email</th>
                  <th className="px-4 py-3">Status</th>
                  <th className="px-4 py-3">Created</th>
                  <th className="px-4 py-3">Last Login</th>
                  <th className="px-4 py-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-200">
                {filteredUsers.map((user) => (
                  <tr key={user.id} className="hover:bg-slate-50/50">
                    <td className="px-4 py-3 font-medium">{user.email}</td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ring-1 ring-inset ${
                        user.status === 'Active' 
                          ? 'bg-green-50 text-green-700 ring-green-600/20' 
                          : 'bg-slate-50 text-slate-700 ring-slate-600/20'
                      }`}>
                        {user.status}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-slate-500">
                      {new Date(user.created_at).toLocaleDateString()}
                    </td>
                    <td className="px-4 py-3 text-slate-500">
                      {user.last_login ? new Date(user.last_login).toLocaleDateString() : '-'}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <Button variant="ghost" size="icon">
                        <MoreHorizontal className="w-4 h-4" />
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
