import { useTranslation } from 'react-i18next';
import { Button } from './ui/Button';
import { Languages } from 'lucide-react';

export function LanguageToggle() {
  const { i18n } = useTranslation();

  const toggleLanguage = () => {
    const newLang = i18n.language === 'zh' ? 'en' : 'zh';
    i18n.changeLanguage(newLang);
    localStorage.setItem('language', newLang);
  };

  return (
    <Button variant="ghost" size="sm" onClick={toggleLanguage} className="w-9 px-0">
      <Languages className="h-4 w-4" />
      <span className="sr-only">Toggle language</span>
    </Button>
  );
}
