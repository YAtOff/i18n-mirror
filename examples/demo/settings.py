"""
Django settings for the i18n pipeline demo.

Minimal single-page demo: the interesting parts are LOCALE_PATHS,
LANGUAGES and LocaleMiddleware.
"""

from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

SECRET_KEY = "demo-only-not-secret"
DEBUG = True
ALLOWED_HOSTS = ["localhost", "127.0.0.1"]

INSTALLED_APPS = [
    "shop",
]

MIDDLEWARE = [
    "django.middleware.security.SecurityMiddleware",
    "django.contrib.sessions.middleware.SessionMiddleware",
    "django.middleware.locale.LocaleMiddleware",
    "django.middleware.common.CommonMiddleware",
    "django.middleware.csrf.CsrfViewMiddleware",
]

# Sessions are used by LocaleMiddleware to remember the selected language.
# Keep them in memory; this demo has no database.
SESSION_ENGINE = "django.contrib.sessions.backends.signed_cookies"

ROOT_URLCONF = "demo.urls"

TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": [],
        "APP_DIRS": True,
        "OPTIONS": {
            "context_processors": [
                "django.template.context_processors.request",
                "django.template.context_processors.i18n",
            ],
        },
    },
]

WSGI_APPLICATION = "demo.wsgi.application"

# --- i18n -----------------------------------------------------------------

# Point both makemessages domains at the project-level locale directory.
LOCALE_PATHS = [BASE_DIR / "locale"]

LANGUAGE_CODE = "en-us"

# Limit the picker to the two languages the demo ships translations for.
LANGUAGES = [
    ("en", "English"),
    ("bg", "Bulgarian"),
]

USE_I18N = True
USE_TZ = True

TIME_ZONE = "Europe/Sofia"

STATIC_URL = "static/"

DEFAULT_AUTO_FIELD = "django.db.models.BigAutoField"
