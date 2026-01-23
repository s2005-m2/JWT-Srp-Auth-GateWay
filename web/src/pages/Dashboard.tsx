import { useTranslation } from 'react-i18next';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/Card';
import { Activity, Users, Globe, Clock } from 'lucide-react';

export default function Dashboard() {
  const { t } = useTranslation();

  const stats = [
    {
      title: t('dashboard.activeUsers'),
      value: "1,234",
      icon: Users,
      change: "+12%",
      color: "text-blue-600"
    },
    {
      title: t('dashboard.totalRequests'),
      value: "843.2K",
      icon: Globe,
      change: "+5.4%",
      color: "text-green-600"
    },
    {
      title: t('dashboard.systemStatus'),
      value: "Healthy",
      icon: Activity,
      color: "text-emerald-600"
    },
    {
      title: t('dashboard.uptime'),
      value: "99.9%",
      icon: Clock,
      color: "text-indigo-600"
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
        {stats.map((stat, index) => {
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
                <div className="text-2xl font-bold">{stat.value}</div>
                {stat.change && (
                  <p className="text-xs text-slate-500 mt-1">
                    <span className="text-green-600 font-medium">{stat.change}</span> from last month
                  </p>
                )}
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
              {[1, 2, 3].map((i) => (
                <div key={i} className="flex items-center">
                  <div className="h-9 w-9 rounded-full bg-slate-100 flex items-center justify-center">
                    <Activity className="h-4 w-4 text-slate-500" />
                  </div>
                  <div className="ml-4 space-y-1">
                    <p className="text-sm font-medium leading-none">New user registered</p>
                    <p className="text-xs text-slate-500">2 minutes ago</p>
                  </div>
                  <div className="ml-auto font-medium text-sm text-green-600">Success</div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
