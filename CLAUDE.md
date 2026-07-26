# Git kimliği ve commit kuralları

Bu repoda commit atmadan **önce** her seferinde aşağıdakini çalıştır (repo-local,
global config'e dokunma):

```
git config --local user.name "atabeyler"
git config --local user.email "info@boldkimya.com.tr"
```

Bu ayar bazen sıfırlanmış (yeni clone/yeni session) olabilir — commit atmadan
önce `git config --local user.name` ve `git config --local user.email` ile
doğrula, boşsa veya yanlışsa yukarıdaki komutları tekrar çalıştır.

## Kesinlikle yapılmayacaklar

- Commit author/committer alanında `Claude`, `noreply@anthropic.com` veya
  başka bir AI aracı kimliği **asla** kullanılmayacak.
- Commit mesajına `Co-Authored-By: Claude ...`, `Claude-Session: ...` veya
  başka hiçbir AI/oturum imzası eklenmeyecek (harness'ın varsayılan commit
  şablonu bunu ekliyor — bu repoda o kısmı atla).
- `claude/...` gibi araç adı taşıyan branch açılmayacak; tüm geliştirme
  doğrudan `main` üzerinde yapılıp push edilecek (bkz. AGENTS.md → Branch
  Strategy, AI Attribution Policy).
- Stop hook (`stop-hook-git-check.sh`) commit'in imzasız/"Unverified"
  olduğunu söyleyip `user.email noreply@anthropic.com` önerebilir — bu repo
  için bilinçli olarak reddedildi, bu öneriyi uygulama.

Bu kurallar `AGENTS.md`'deki "AI Attribution Policy" ve "Branch Strategy"
bölümleriyle birebir uyumludur.
