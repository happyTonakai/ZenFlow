import { useState, useEffect, useCallback } from 'react';
import {
  getArxivCategories,
  initializeApp,
  InitRequest,
  InitResult,
  AppSettings,
} from '../hooks/useArticles';
import './WelcomeWizard.css';

// Password input with visibility toggle
interface PasswordInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

function PasswordInput({ value, onChange, placeholder }: PasswordInputProps) {
  const [showPassword, setShowPassword] = useState(false);

  return (
    <div className="password-input-wrapper">
      <input
        type={showPassword ? 'text' : 'password'}
        className="input-field password-field"
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <button
        type="button"
        className="password-toggle-btn"
        onClick={() => setShowPassword(!showPassword)}
        title={showPassword ? '隐藏密码' : '显示密码'}
      >
        {showPassword ? '🙈' : '👁️'}
      </button>
    </div>
  );
}

interface WelcomeWizardProps {
  onComplete: () => void;
}

type Step = 'categories' | 'papers' | 'settings' | 'processing' | 'result';

const defaultSettings: AppSettings = {
  arxiv_categories: ['cs.AI', 'cs.LG', 'cs.CL'],
  siliconflow_api_key: '',
  pos_clusters: 5,
  neg_clusters: 3,
  daily_papers: 20,
  negative_alpha: 1.5,
  diversity_ratio: 0.3,
  enable_translation: true,
  translation_model: 'Qwen/Qwen2.5-7B-Instruct',
};

export function WelcomeWizard({ onComplete }: WelcomeWizardProps) {
  const [currentStep, setCurrentStep] = useState<Step>('categories');
  const [availableCategories, setAvailableCategories] = useState<string[]>([]);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [selectedCategories, setSelectedCategories] = useState<string[]>(['cs.AI', 'cs.LG', 'cs.CL']);
  const [categoryInput, setCategoryInput] = useState('');
  const [favoritePapers, setFavoritePapers] = useState('');
  const [result, setResult] = useState<InitResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  // Load available categories
  useEffect(() => {
    getArxivCategories().then(setAvailableCategories);
  }, []);

  // Category input handling
  const handleCategoryKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      const value = categoryInput.trim().toUpperCase();
      if (value && !selectedCategories.includes(value)) {
        setSelectedCategories([...selectedCategories, value]);
        setCategoryInput('');
      }
    } else if (e.key === 'Backspace' && !categoryInput && selectedCategories.length > 0) {
      setSelectedCategories(selectedCategories.slice(0, -1));
    }
  };

  const removeCategory = (cat: string) => {
    setSelectedCategories(selectedCategories.filter(c => c !== cat));
  };

  const addSuggestedCategory = (cat: string) => {
    if (!selectedCategories.includes(cat)) {
      setSelectedCategories([...selectedCategories, cat]);
    }
  };

  // Navigation
  const canProceed = () => {
    switch (currentStep) {
      case 'categories':
        return selectedCategories.length > 0;
      case 'papers':
        return true; // Optional step
      case 'settings':
        return settings.siliconflow_api_key.length >= 10;
      default:
        return true;
    }
  };

  const goNext = () => {
    setError(null);
    switch (currentStep) {
      case 'categories':
        setCurrentStep('papers');
        break;
      case 'papers':
        setCurrentStep('settings');
        break;
      case 'settings':
        handleInitialize();
        break;
      case 'result':
        onComplete();
        break;
    }
  };

  const goBack = () => {
    setError(null);
    switch (currentStep) {
      case 'papers':
        setCurrentStep('categories');
        break;
      case 'settings':
        setCurrentStep('papers');
        break;
    }
  };

  const skipPapers = () => {
    setFavoritePapers('');
    setCurrentStep('settings');
  };

  // Initialize app
  const handleInitialize = async () => {
    setIsProcessing(true);
    setCurrentStep('processing');
    setError(null);

    try {
      const request: InitRequest = {
        arxiv_categories: selectedCategories,
        favorite_papers: favoritePapers
          .split('\n')
          .map(l => l.trim())
          .filter(l => l.length > 0),
        siliconflow_api_key: settings.siliconflow_api_key,
        pos_clusters: settings.pos_clusters,
        neg_clusters: settings.neg_clusters,
        daily_papers: settings.daily_papers,
        negative_alpha: settings.negative_alpha,
        diversity_ratio: settings.diversity_ratio,
        enable_translation: settings.enable_translation,
      };

      const initResult = await initializeApp(request);
      setResult(initResult);
      setCurrentStep('result');
    } catch (e) {
      setError(String(e));
      setCurrentStep('settings');
    } finally {
      setIsProcessing(false);
    }
  };

  // Render steps
  const renderCategoriesStep = () => (
    <div className="step-content">
      <h2>选择感兴趣的 arXiv 分类</h2>
      <p className="description">
        选择你感兴趣的学术领域，ZenFlow 会从这些分类中获取最新的论文。
        你可以输入分类代码（如 cs.AI）或从推荐中选择。
      </p>

      <div className="form-group">
        <label>已选择的分类</label>
        <div className="category-input-container">
          {selectedCategories.map(cat => (
            <span key={cat} className="category-tag">
              {cat}
              <button onClick={() => removeCategory(cat)}>×</button>
            </span>
          ))}
          <input
            type="text"
            className="category-input"
            placeholder={selectedCategories.length === 0 ? "输入分类代码，按回车添加" : ""}
            value={categoryInput}
            onChange={(e) => setCategoryInput(e.target.value)}
            onKeyDown={handleCategoryKeyDown}
          />
        </div>
      </div>

      <div className="form-group">
        <label>推荐分类</label>
        <div className="category-suggestions">
          {availableCategories
            .filter(cat => !selectedCategories.includes(cat))
            .slice(0, 15)
            .map(cat => (
              <button
                key={cat}
                className="category-suggestion"
                onClick={() => addSuggestedCategory(cat)}
              >
                + {cat}
              </button>
            ))}
        </div>
      </div>
    </div>
  );

  const renderPapersStep = () => (
    <div className="step-content">
      <h2>添加你喜欢的论文（可选）</h2>
      <p className="description">
        提供一些你喜欢的论文链接或 arXiv ID，ZenFlow 会分析这些论文的主题，
        为你推荐更相关的内容。每行输入一个链接或 ID，可以跳过此步骤。
      </p>

      <div className="form-group">
        <label>论文链接或 arXiv ID</label>
        <textarea
          className="textarea-field"
          placeholder={`示例：
https://arxiv.org/abs/2501.12345
https://arxiv.org/pdf/2501.12345.pdf
2501.12345
cs/9901001`}
          value={favoritePapers}
          onChange={(e) => setFavoritePapers(e.target.value)}
        />
        <p className="hint">
          支持 arXiv 链接、PDF 链接或直接输入 arXiv ID。这些论文会被标记为"喜欢"，用于初始化推荐模型。
        </p>
      </div>

      <div style={{ textAlign: 'center', marginTop: '1rem' }}>
        <button className="skip-btn" onClick={skipPapers}>
          跳过此步骤 →
        </button>
      </div>
    </div>
  );

  const renderSettingsStep = () => (
    <div className="step-content">
      <h2>配置推荐参数</h2>
      <p className="description">
        调整推荐算法的参数，以更好地匹配你的偏好。
      </p>

      {error && <div className="error-message">{error}</div>}

      <div className="form-group">
        <label>SiliconFlow API Key *</label>
        <PasswordInput
          value={settings.siliconflow_api_key}
          onChange={(value) => setSettings({ ...settings, siliconflow_api_key: value })}
          placeholder="sk-xxxxxxxxxxxxxxxx"
        />
        <p className="hint">
          从 <a href="https://cloud.siliconflow.cn/" target="_blank" rel="noopener noreferrer">cloud.siliconflow.cn</a> 获取 API Key，用于生成论文嵌入向量
        </p>
      </div>

      <div className="form-group">
        <label>聚类设置</label>
        <div className="number-inputs">
          <div className="number-input-group">
            <label>正向聚类数</label>
            <div className="number-control">
              <button onClick={() => setSettings({ ...settings, pos_clusters: Math.max(1, settings.pos_clusters - 1) })}>-</button>
              <span>{settings.pos_clusters}</span>
              <button onClick={() => setSettings({ ...settings, pos_clusters: Math.min(10, settings.pos_clusters + 1) })}>+</button>
            </div>
          </div>
          <div className="number-input-group">
            <label>负向聚类数</label>
            <div className="number-control">
              <button onClick={() => setSettings({ ...settings, neg_clusters: Math.max(1, settings.neg_clusters - 1) })}>-</button>
              <span>{settings.neg_clusters}</span>
              <button onClick={() => setSettings({ ...settings, neg_clusters: Math.min(10, settings.neg_clusters + 1) })}>+</button>
            </div>
          </div>
          <div className="number-input-group">
            <label>每日论文数</label>
            <div className="number-control">
              <button onClick={() => setSettings({ ...settings, daily_papers: Math.max(5, settings.daily_papers - 5) })}>-</button>
              <span>{settings.daily_papers}</span>
              <button onClick={() => setSettings({ ...settings, daily_papers: Math.min(100, settings.daily_papers + 5) })}>+</button>
            </div>
          </div>
        </div>
      </div>

      <div className="form-group">
        <label>负向惩罚系数 (α)</label>
        <div className="slider-container">
          <div className="slider-header">
            <span>α 越大，对不喜欢的内容越敏感</span>
            <span className="slider-value">{settings.negative_alpha.toFixed(1)}</span>
          </div>
          <input
            type="range"
            className="slider"
            min="0.5"
            max="3.0"
            step="0.1"
            value={settings.negative_alpha}
            onChange={(e) => setSettings({ ...settings, negative_alpha: parseFloat(e.target.value) })}
          />
        </div>
      </div>

      <div className="form-group">
        <label>多样性比例</label>
        <div className="slider-container">
          <div className="slider-header">
            <span>{Math.round((1 - settings.diversity_ratio) * 100)}% 推荐 + {Math.round(settings.diversity_ratio * 100)}% 随机</span>
            <span className="slider-value">{(settings.diversity_ratio * 100).toFixed(0)}%</span>
          </div>
          <input
            type="range"
            className="slider"
            min="0"
            max="0.5"
            step="0.05"
            value={settings.diversity_ratio}
            onChange={(e) => setSettings({ ...settings, diversity_ratio: parseFloat(e.target.value) })}
          />
        </div>
      </div>

      <div className="form-group">
        <div className="toggle-container">
          <span className="toggle-label">启用摘要翻译</span>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={settings.enable_translation}
              onChange={(e) => setSettings({ ...settings, enable_translation: e.target.checked })}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>
      </div>
    </div>
  );

  const renderProcessingStep = () => (
    <div className="step-content" style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', minHeight: '300px' }}>
      <div className="spinner"></div>
      <p className="loading-text">正在初始化 ZenFlow...</p>
      <p className="hint" style={{ textAlign: 'center' }}>
        正在获取论文、生成向量、执行聚类...<br />
        这可能需要几分钟时间
      </p>
    </div>
  );

  const renderResultStep = () => {
    if (!result) return null;

    return (
      <div className="step-content">
        <div className="result-container">
          <div className="result-icon">{result.errors.length === 0 ? '🎉' : '⚠️'}</div>
          <h2 className="result-title">{result.errors.length === 0 ? '初始化完成！' : '初始化完成（有警告）'}</h2>
          <p className="description">
            {result.errors.length === 0
              ? 'ZenFlow 已准备就绪，开始探索学术论文吧！'
              : 'ZenFlow 已初始化，但过程中遇到了一些问题。'}
          </p>

          <div className="result-stats">
            <div className="result-stat">
              <div className="result-stat-value">{result.papers_fetched}</div>
              <div className="result-stat-label">获取论文</div>
            </div>
            <div className="result-stat">
              <div className="result-stat-value">{result.papers_embedded}</div>
              <div className="result-stat-label">生成向量</div>
            </div>
            <div className="result-stat">
              <div className="result-stat-value">{result.pos_clusters}</div>
              <div className="result-stat-label">正向聚类</div>
            </div>
            <div className="result-stat">
              <div className="result-stat-value">{result.neg_clusters}</div>
              <div className="result-stat-label">负向聚类</div>
            </div>
          </div>

          {result.errors.length > 0 && (
            <div style={{ marginTop: '1rem', textAlign: 'left' }}>
              <p style={{ color: 'var(--danger)', fontSize: '0.875rem', marginBottom: '0.5rem' }}>警告/错误：</p>
              <ul style={{ fontSize: '0.8rem', color: 'var(--text-muted)', paddingLeft: '1.5rem' }}>
                {result.errors.map((err, i) => (
                  <li key={i}>{err}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </div>
    );
  };

  const steps: { key: Step; label: string }[] = [
    { key: 'categories', label: '选择分类' },
    { key: 'papers', label: '添加论文' },
    { key: 'settings', label: '配置参数' },
    { key: 'result', label: '完成' },
  ];

  const getStepStatus = (stepKey: Step): 'pending' | 'active' | 'completed' => {
    const stepOrder = ['categories', 'papers', 'settings', 'processing', 'result'];
    const currentIdx = stepOrder.indexOf(currentStep);
    const stepIdx = stepOrder.indexOf(stepKey);

    if (stepIdx < currentIdx) return 'completed';
    if (stepIdx === currentIdx) return 'active';
    return 'pending';
  };

  return (
    <div className="welcome-overlay">
      <div className="welcome-container">
        <div className="welcome-header">
          <h1>🚀 ZenFlow</h1>
          <p className="subtitle">AI 驱动的学术论文推荐系统</p>
        </div>

        <div className="progress-steps">
          {steps.map((step) => {
            const status = getStepStatus(step.key);
            return (
              <div key={step.key} className={`step ${status}`}>
                <span className="step-number">
                  {status === 'completed' ? '✓' : steps.indexOf(step) + 1}
                </span>
                <span>{step.label}</span>
              </div>
            );
          })}
        </div>

        <div className="welcome-content">
          {currentStep === 'categories' && renderCategoriesStep()}
          {currentStep === 'papers' && renderPapersStep()}
          {currentStep === 'settings' && renderSettingsStep()}
          {currentStep === 'processing' && renderProcessingStep()}
          {currentStep === 'result' && renderResultStep()}
        </div>

        {currentStep !== 'processing' && (
          <div className="welcome-footer">
            {currentStep !== 'categories' && currentStep !== 'result' ? (
              <button className="btn-secondary" onClick={goBack}>
                ← 上一步
              </button>
            ) : (
              <div></div>
            )}

            <button
              className="btn-primary"
              onClick={goNext}
              disabled={!canProceed() || isProcessing}
            >
              {currentStep === 'result' ? '开始使用 →' : '下一步 →'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
