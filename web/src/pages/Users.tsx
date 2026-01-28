import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Search, UserPlus, Ban, CheckCircle, Trash2 } from 'lucide-react';
import { adminApi } from '../lib/api';

interface User {
  id: string;
  email: string;
  email_verified: boolean;
  is_active: boolean;
  created_at: string;
}

export default function Users() {
  const { t } = useTranslation();
  const [users, setUsers] = useState<User[]>([]);
  const [search, setSearch] = useState('');

  const fetchUsers = async () => {
    const res = await adminApi.getUsers();
    if (res.data) {
      setUsers(res.data.users || []);
    }
  };

  useEffect(() => {
    fetchUsers();
  }, []);

  const handleToggleStatus = async (user: User) => {
    const res = await adminApi.updateUserStatus(user.id, !user.is_active);
    if (res.data?.success) {
      fetchUsers();
    }
  };

  const handleDelete = async (user: User) => {
    if (!confirm(`Delete user ${user.email}?`)) return;
    const res = await adminApi.deleteUser(user.id);
    if (res.data?.success) {
      fetchUsers();
    }
  };

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
                  <th className="px-4 py-3">Verified</th>
                  <th className="px-4 py-3">Status</th>
                  <th className="px-4 py-3">Created</th>
                  <th className="px-4 py-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-200">
                {filteredUsers.map((user) => (
                  <tr key={user.id} className="hover:bg-slate-50/50">
                    <td className="px-4 py-3 font-medium">{user.email}</td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ring-1 ring-inset ${
                        user.email_verified 
                          ? 'bg-green-50 text-green-700 ring-green-600/20' 
                          : 'bg-yellow-50 text-yellow-700 ring-yellow-600/20'
                      }`}>
                        {user.email_verified ? 'Verified' : 'Pending'}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex items-center rounded-full px-2 py-1 text-xs font-medium ring-1 ring-inset ${
                        user.is_active 
                          ? 'bg-blue-50 text-blue-700 ring-blue-600/20' 
                          : 'bg-red-50 text-red-700 ring-red-600/20'
                      }`}>
                        {user.is_active ? 'Active' : 'Disabled'}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-slate-500">
                      {new Date(user.created_at).toLocaleDateString()}
                    </td>
                    <td className="px-4 py-3 text-right space-x-1">
                      <Button 
                        variant="ghost" 
                        size="icon"
                        onClick={() => handleToggleStatus(user)}
                        title={user.is_active ? 'Disable' : 'Enable'}
                      >
                        {user.is_active ? <Ban className="w-4 h-4" /> : <CheckCircle className="w-4 h-4" />}
                      </Button>
                      <Button 
                        variant="ghost" 
                        size="icon"
                        onClick={() => handleDelete(user)}
                        title="Delete"
                      >
                        <Trash2 className="w-4 h-4 text-red-500" />
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
