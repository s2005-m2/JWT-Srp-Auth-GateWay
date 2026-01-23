import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Activity, Users, Globe } from 'lucide-react';
import { useAuth } from '../context/AuthContext';

interface Stats {
  active_users: number;
  total_requests: number;
  system_status: string;
}

interface ActivityItem {
  id: string;
  action: string;
  email: string;
  status: string;
  created_at: string;
}

export default function Dashboard() {
  const { t } = useTranslation();
  const { token } = useAuth();
  const [stats, setStats] = useState<Stats | null>(null);
  const [activities, setActivities] = useState<ActivityItem[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const headers = { Authorization: `Bearer ${token}` };
        const [statsRes, activitiesRes] = await Promise.all([
          fetch('/api/admin/stats', { headers }),
          fetch('/api/admin/activities', { headers }),
        ]);
        if (statsRes.ok) setStats(await statsRes.json());
        if (activitiesRes.ok) {
          const data = await activitiesRes.json();
          setActivities(data.activities || []);
        }
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, [token]);

  const statCards = [
    {
      title: t('dashboard.activeUsers'),
      value: stats?.active_users?.toLocaleString() ?? '-',
      icon: Users,
      color: "text-blue-600"
    },
    {
      title: t('dashboard.totalRequests'),
      value: stats?.total_requests?.toLocaleString() ?? '-',
      icon: Globe,
      color: "text-green-600"
    },
    {
      title: t('dashboard.systemStatus'),
      value: stats?.system_status ?? '-',
      icon: Activity,
      color: "text-emerald-600"
    }
  ];

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-3xl font-bold tracking-tight">{t('common.dashboard')}</h2>
        <p className="text-muted-foreground text-slate-500 mt-2">
          {t('dashboard.welcome')}
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {statCards.map((stat, index) => {
          const Icon = stat.icon;
          return (
            <Card key={index}>
              <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium text-slate-500">
                  {stat.title}
                </CardTitle>
                <Icon className={`h-4 w-4 ${stat.color}`} />
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold">{loading ? '...' : stat.value}</div>
              </CardContent>
            </Card>
          );
        })}
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        <Card className="col-span-4">
          <CardHeader>
            <CardTitle>Request Traffic</CardTitle>
          </CardHeader>
          <CardContent className="pl-2">
            <div className="h-[200px] flex items-center justify-center text-slate-400 bg-slate-50 rounded-md border border-dashed border-slate-200">
              Chart Placeholder
            </div>
          </CardContent>
        </Card>
        <Card className="col-span-3">
          <CardHeader>
            <CardTitle>Recent Activity</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-8">
              {activities.length === 0 && !loading && (
                <p className="text-sm text-slate-500">No recent activity</p>
              )}
              {activities.slice(0, 5).map((item) => (
                <div key={item.id} className="flex items-center">
                  <div className="h-9 w-9 rounded-full bg-slate-100 flex items-center justify-center">
                    <Activity className="h-4 w-4 text-slate-500" />
                  </div>
                  <div className="ml-4 space-y-1">
                    <p className="text-sm font-medium leading-none">{item.action}</p>
                    <p className="text-xs text-slate-500">{item.email}</p>
                  </div>
                  <div className="ml-auto font-medium text-sm text-green-600">{item.status}</div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
