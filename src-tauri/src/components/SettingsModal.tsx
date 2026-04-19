import { useState, useEffect } from 'react';
import { AppSettings, getSettings, saveSettings, requestKeychainAccess } from '../hooks/useArticles';
import './SettingsModal.css';

interface SettingsModalProps {
  onClose: () => void;
  onSave?: () => void;
}

type TabKey = 'general' | 'scoring' | 'translation' | 'algorithm';

// Simple password input component
function PasswordInput({ value, onChange, placeholder }: { value: string, onChange: (val: string) => void, placeholder?: string }) {
  const [show, setShow] = useState(false);
  return (
    <div className="settings-password-wrapper">
      <input
        type={show ? 'text' : 'password'}
        className="settings-input"
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <button
        type="button"
        className="settings-password-toggle"
        onClick={() => setShow(!show)}
        title={show ? '隐藏密码' : '显示密码'}
      >
        {show ? (
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>
        ) : (
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path><line x1="1" y1="1" x2="23" y2="23"></line></svg>
        )}
      </button>
    </div>
  );
}

export function SettingsModal({ onClose, onSave }: SettingsModalProps) {
  const [activeTab, setActiveTab] = useState<TabKey>('general');
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Category input state
  const [catInput, setCatInput] = useState('');

  useEffect(() => {
    getSettings().then(setSettings).catch(e => setError(String(e)));
  }, []);

  if (!settings) return null;

  const handleSave = async () => {
    try {
      setIsSaving(true);
      setError(null);
      if (settings.scoring_api_key.length > 5) {
        await requestKeychainAccess(settings.scoring_api_key);
      }

      await saveSettings(settings);
      onSave?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setIsSaving(false);
    }
  };

  const handleCatKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      const val = catInput.trim().toUpperCase();
      if (val && !settings.arxiv_categories.includes(val)) {
        setSettings({ ...settings, arxiv_categories: [...settings.arxiv_categories, val] });
        setCatInput('');
      } else if (!val) {
        setCatInput('');
      }
    }
  };

  const removeCat = (cat: string) => {
    setSettings({
      ...settings,
      arxiv_categories: settings.arxiv_categories.filter(c => c !== cat)
    });
  };

  return (
    <div className="settings-overlay" onMouseDown={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}>
      <div className="settings-container">
        <div className="settings-header">
          <h2>设置</h2>
          <button className="close-btn" onClick={onClose}>x</button>
        </div>

        <div className="settings-body">
          <div className="settings-sidebar">
            <button className={`settings-tab ${activeTab === 'general' ? 'active' : ''}`} onClick={() => setActiveTab('general')}>
              基础设置
            </button>
            <button className={`settings-tab ${activeTab === 'scoring' ? 'active' : ''}`} onClick={() => setActiveTab('scoring')}>
              推荐 API
            </button>
            <button className={`settings-tab ${activeTab === 'translation' ? 'active' : ''}`} onClick={() => setActiveTab('translation')}>
              翻译 API
            </button>
            <button className={`settings-tab ${activeTab === 'algorithm' ? 'active' : ''}`} onClick={() => setActiveTab('algorithm')}>
              推荐算法
            </button>
          </div>

          <div className="settings-content">
            {error && <div className="error-message" style={{ marginBottom: '1rem', color: 'var(--danger)' }}>{error}</div>}

            {activeTab === 'general' && (
              <div className="tab-pane">
                <h3>基础设置</h3>

                <div className="settings-form-group">
                  <label>arXiv 订阅分类</label>
                  <div className="settings-categories">
                    {settings.arxiv_categories.map(cat => (
                      <span key={cat} className="settings-category-tag">
                        {cat} <button onClick={() => removeCat(cat)}>x</button>
                      </span>
                    ))}
                  </div>
                  <input
                    type="text"
                    className="settings-input"
                    placeholder="输入分类代码，按回车添加 (例如: CS.AI)"
                    value={catInput}
                    onChange={(e) => setCatInput(e.target.value)}
                    onKeyDown={handleCatKeyDown}
                  />
                  <p className="hint">设置你想跟踪的 arXiv 分类，如 CS.LG, CS.CV 等</p>
                </div>

                <div className="settings-form-group">
                  <label>每日论文获取数量</label>
                  <input
                    type="number"
                    className="settings-input"
                    style={{ width: '150px' }}
                    min="1" max="100"
                    value={settings.daily_papers}
                    onChange={(e) => setSettings({...settings, daily_papers: parseInt(e.target.value) || 20})}
                  />
                  <p className="hint">每天展示的最大论文数量（推荐 10-50）</p>
                </div>

                <div className="settings-form-group">
                  <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
                    <input
                      type="checkbox"
                      checked={settings.auto_refresh_recommendations}
                      onChange={(e) => setSettings({...settings, auto_refresh_recommendations: e.target.checked})}
                    />
                    自动刷新推荐
                  </label>
                  <p className="hint">开启后，应用启动时会自动抓取 RSS 并生成每日推荐（需要推荐 API 已配置）</p>
                </div>
              </div>
            )}

            {activeTab === 'scoring' && (
              <div className="tab-pane">
                <h3>推荐 API</h3>
                <p className="hint" style={{ marginBottom: '1.5rem' }}>用于论文评分和用户偏好分析的 LLM API（OpenAI 兼容格式）。</p>

                <div className="settings-form-group">
                  <label>API Key</label>
                  <PasswordInput
                    value={settings.scoring_api_key}
                    onChange={(val) => setSettings({...settings, scoring_api_key: val})}
                    placeholder="sk-..."
                  />
                </div>

                <div className="settings-form-group">
                  <label>Base URL</label>
                  <input
                    type="text"
                    className="settings-input"
                    value={settings.scoring_api_base_url}
                    onChange={(e) => setSettings({...settings, scoring_api_base_url: e.target.value})}
                  />
                </div>

                <div className="settings-form-group">
                  <label>模型名称 (Model)</label>
                  <input
                    type="text"
                    className="settings-input"
                    value={settings.scoring_model}
                    onChange={(e) => setSettings({...settings, scoring_model: e.target.value})}
                  />
                </div>
              </div>
            )}

            {activeTab === 'translation' && (
              <div className="tab-pane">
                <h3>翻译 API</h3>
                <p className="hint" style={{ marginBottom: '1.5rem' }}>用于将论文标题和摘要翻译为中文。如果留空，则不进行翻译。</p>

                <div className="settings-form-group">
                  <label>API Key</label>
                  <PasswordInput
                    value={settings.translation_api_key}
                    onChange={(val) => setSettings({...settings, translation_api_key: val})}
                    placeholder="sk-..."
                  />
                </div>

                <div className="settings-form-group">
                  <label>Base URL</label>
                  <input
                    type="text"
                    className="settings-input"
                    value={settings.translation_api_base_url}
                    onChange={(e) => setSettings({...settings, translation_api_base_url: e.target.value})}
                  />
                </div>

                <div className="settings-form-group">
                  <label>模型名称 (Model)</label>
                  <input
                    type="text"
                    className="settings-input"
                    value={settings.translation_model}
                    onChange={(e) => setSettings({...settings, translation_model: e.target.value})}
                  />
                </div>
              </div>
            )}

            {activeTab === 'algorithm' && (
              <div className="tab-pane">
                <h3>推荐算法</h3>
                <p className="hint" style={{ marginBottom: '1.5rem' }}>调整 ZenFlow 推荐结果的倾向性。</p>

                <div className="settings-form-group">
                  <label>多样性比例</label>
                  <div className="settings-slider-row">
                    <input
                      type="range"
                      min="0" max="0.5" step="0.05"
                      value={settings.diversity_ratio}
                      onChange={(e) => setSettings({...settings, diversity_ratio: parseFloat(e.target.value)})}
                    />
                    <span className="settings-slider-value">{Math.round(settings.diversity_ratio * 100)}%</span>
                  </div>
                  <p className="hint">加入一定比例的随机内容，帮助你跳出信息茧房、探索新领域。</p>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="settings-footer">
          <button className="btn-secondary" onClick={onClose} disabled={isSaving}>取消</button>
          <button className="btn-primary" onClick={handleSave} disabled={isSaving}>
            {isSaving ? '保存中...' : '保存更改'}
          </button>
        </div>
      </div>
    </div>
  );
}
