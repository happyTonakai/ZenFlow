import { useState, useEffect } from 'react';
import { AppSettings, getSettings, saveSettings, requestKeychainAccess } from '../hooks/useArticles';
import './SettingsModal.css';

interface SettingsModalProps {
  onClose: () => void;
  onSave?: () => void;
}

type TabKey = 'general' | 'embedding' | 'translation' | 'algorithm';

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
      // Ensure keychain access if siliconflow key or embedding key changes
      // In a real scenario we'd track if it changed, but it's safe to just request it
      if (settings.embedding_api_key.length > 5) {
        await requestKeychainAccess(settings.embedding_api_key);
      } else if (settings.siliconflow_api_key.length > 5) { // Fallback to old field
        await requestKeychainAccess(settings.siliconflow_api_key);
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
          <h2>⚙️ 设置</h2>
          <button className="close-btn" onClick={onClose}>×</button>
        </div>

        <div className="settings-body">
          <div className="settings-sidebar">
            <button className={`settings-tab ${activeTab === 'general' ? 'active' : ''}`} onClick={() => setActiveTab('general')}>
              📋 基础设置
            </button>
            <button className={`settings-tab ${activeTab === 'embedding' ? 'active' : ''}`} onClick={() => setActiveTab('embedding')}>
              🧠 嵌入 API
            </button>
            <button className={`settings-tab ${activeTab === 'translation' ? 'active' : ''}`} onClick={() => setActiveTab('translation')}>
              🌐 翻译 API
            </button>
            <button className={`settings-tab ${activeTab === 'algorithm' ? 'active' : ''}`} onClick={() => setActiveTab('algorithm')}>
              🎛️ 推荐算法
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
                        {cat} <button onClick={() => removeCat(cat)}>×</button>
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
              </div>
            )}

            {activeTab === 'embedding' && (
              <div className="tab-pane">
                <h3>嵌入 API</h3>
                <p className="hint" style={{ marginBottom: '1.5rem' }}>用于将论文摘要转换为向量以计算相似度。推荐使用 SiliconFlow 提供的 BAAI/bge-m3 模型。</p>
                
                <div className="settings-form-group">
                  <label>API Key</label>
                  <PasswordInput 
                    value={settings.embedding_api_key || settings.siliconflow_api_key} 
                    onChange={(val) => setSettings({...settings, embedding_api_key: val})} 
                    placeholder="sk-..." 
                  />
                </div>
                
                <div className="settings-form-group">
                  <label>Base URL</label>
                  <input 
                    type="text" 
                    className="settings-input"
                    value={settings.embedding_api_base_url}
                    onChange={(e) => setSettings({...settings, embedding_api_base_url: e.target.value})}
                  />
                </div>

                <div className="settings-form-group">
                  <label>模型名称 (Model)</label>
                  <input 
                    type="text" 
                    className="settings-input"
                    value={settings.embedding_model}
                    onChange={(e) => setSettings({...settings, embedding_model: e.target.value})}
                  />
                  <p className="hint" style={{ color: 'var(--accent)', marginTop: '0.5rem' }}>
                    ⚠️ 注意：如果修改了此向量模型，数据库中以往保存的所有文章（包括偏好文章）都需要重新提取计算向量，这会有一次性的计算量并且可能产生兼容问题。
                  </p>
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
                
                <div className="settings-param-grid">
                  <div className="settings-param-cell">
                    <label>正向聚类数</label>
                    <input 
                      type="number" 
                      className="settings-input" 
                      min="1" max="50"
                      value={settings.pos_clusters}
                      onChange={(e) => setSettings({...settings, pos_clusters: parseInt(e.target.value) || 5})}
                    />
                    <p className="hint" style={{ fontSize: '0.75rem', marginTop: '0.5rem' }}>大值表示兴趣更广</p>
                  </div>
                  <div className="settings-param-cell">
                    <label>负向聚类数</label>
                    <input 
                      type="number" 
                      className="settings-input" 
                      min="1" max="50"
                      value={settings.neg_clusters}
                      onChange={(e) => setSettings({...settings, neg_clusters: parseInt(e.target.value) || 3})}
                    />
                    <p className="hint" style={{ fontSize: '0.75rem', marginTop: '0.5rem' }}>过滤负面内容</p>
                  </div>
                </div>

                <div className="settings-form-group">
                  <label>负向惩罚系数 (α)</label>
                  <div className="settings-slider-row">
                    <input 
                      type="range" 
                      min="0.5" max="3.0" step="0.1"
                      value={settings.negative_alpha}
                      onChange={(e) => setSettings({...settings, negative_alpha: parseFloat(e.target.value)})}
                    />
                    <span className="settings-slider-value">{settings.negative_alpha.toFixed(1)}</span>
                  </div>
                  <p className="hint">α 越大，推荐结果越倾向于避开你不喜欢的内容领域。</p>
                </div>

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
