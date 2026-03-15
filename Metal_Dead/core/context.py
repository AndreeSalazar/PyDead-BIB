"""
Contexto Personal para Metal-Dead
==================================
Author: Eddi Andreé Salazar Matos
Made with ❤️ in Peru 🇵🇪
"""

import json
import time
from pathlib import Path
from typing import List, Dict, Any
from dataclasses import dataclass, field, asdict
from datetime import datetime


@dataclass
class UserProfile:
    name: str = "Usuario"
    language: str = "es"
    interests: List[str] = field(default_factory=list)
    preferences: Dict[str, Any] = field(default_factory=dict)
    interaction_count: int = 0
    first_interaction: float = 0.0
    last_interaction: float = 0.0
    learned_facts: Dict[str, str] = field(default_factory=dict)
    
    def to_dict(self) -> Dict:
        return asdict(self)
    
    @classmethod
    def from_dict(cls, data: Dict) -> "UserProfile":
        return cls(**data)


class PersonalContext:
    def __init__(self, data_dir: str):
        self.data_path = Path(data_dir)
        self.data_path.mkdir(parents=True, exist_ok=True)
        self.profile_file = self.data_path / "profile.json"
        self.profile = UserProfile()
        self._load()
    
    def update_interaction(self):
        now = time.time()
        if self.profile.first_interaction == 0:
            self.profile.first_interaction = now
        self.profile.last_interaction = now
        self.profile.interaction_count += 1
        self._save()
    
    def set_name(self, name: str):
        self.profile.name = name.strip().capitalize()
        self._save()
    
    def add_interest(self, interest: str):
        interest = interest.strip().lower()
        if interest and interest not in [i.lower() for i in self.profile.interests]:
            self.profile.interests.append(interest)
            self._save()
    
    def learn_fact(self, key: str, value: str):
        self.profile.learned_facts[key] = value
        self._save()
    
    def get_greeting(self) -> str:
        hour = datetime.now().hour
        if hour < 12:
            greeting = "Buenos días"
        elif hour < 18:
            greeting = "Buenas tardes"
        else:
            greeting = "Buenas noches"
        
        name = self.profile.name
        count = self.profile.interaction_count
        
        if count == 0:
            return f"¡Hola! Soy Metal-Dead, tu IA personal. ¿Cómo te llamas?"
        elif count < 3:
            return f"{greeting}, {name}. ¿En qué puedo ayudarte?"
        elif count < 10:
            return f"{greeting}, {name}. Me alegra verte de nuevo."
        else:
            if self.profile.interests:
                interest = self.profile.interests[0]
                return f"{greeting}, {name}. ¿Qué tal va todo con {interest}?"
            return f"{greeting}, {name}. ¡Qué bueno verte otra vez!"
    
    def get_summary(self) -> str:
        p = self.profile
        lines = [f"👤 **Perfil de {p.name}**", f"• Interacciones: {p.interaction_count}"]
        if p.interests:
            lines.append(f"• Intereses: {', '.join(p.interests[:5])}")
        if p.first_interaction > 0:
            days = (time.time() - p.first_interaction) / 86400
            lines.append(f"• Te conozco hace: {days:.0f} días")
        return "\n".join(lines)
    
    def _save(self):
        with open(self.profile_file, 'w', encoding='utf-8') as f:
            json.dump(self.profile.to_dict(), f, ensure_ascii=False, indent=2)
    
    def _load(self):
        if self.profile_file.exists():
            try:
                with open(self.profile_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)
                self.profile = UserProfile.from_dict(data)
                print(f"👤 Perfil cargado: {self.profile.name}")
            except Exception as e:
                print(f"⚠️ Error cargando perfil: {e}")
