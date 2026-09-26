---------------------------- MODULE auth ----------------------------
(***************************************************************************)
(* 認証の状態遷移の TLA+ 仕様 (issue 0026、ADR-0013)。                     *)
(*                                                                         *)
(* モデル化するのは、利用者、パスキー、チャレンジ、登録用トークン、        *)
(* セッションである。定数を小さくし、TLC で短時間に検査できるようにする。  *)
(* 実行は `mise run formal` (実装の対応は formal/README.md を参照)。       *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets

CONSTANT Users, Passkeys, Challenges, Tokens, Sessions

VARIABLES passkeys, everHadPasskey, issuedChallenge, consumedChallenge,
          issuedToken, usedToken, tokenOwner, activeSession, endedSession,
          registeredPasskey, tokenOutcome, loginSession

vars == <<passkeys, everHadPasskey, issuedChallenge, consumedChallenge,
          issuedToken, usedToken, tokenOwner, activeSession, endedSession,
          registeredPasskey, tokenOutcome, loginSession>>

TypeOK ==
  /\ passkeys \in [Users -> SUBSET Passkeys]
  /\ everHadPasskey \subseteq Users
  /\ issuedChallenge \subseteq Challenges
  /\ consumedChallenge \subseteq issuedChallenge
  /\ issuedToken \subseteq Tokens
  /\ usedToken \subseteq issuedToken
  (* 登録用トークンは発行先の利用者に紐づく (FR-17)。 *)
  /\ tokenOwner \in [Tokens -> Users]
  /\ activeSession \subseteq Sessions
  /\ endedSession \subseteq Sessions
  /\ registeredPasskey \in [Challenges -> SUBSET Passkeys]
  /\ tokenOutcome \in [Tokens -> SUBSET Passkeys]
  /\ loginSession \in [Challenges -> SUBSET Sessions]

Init ==
  /\ passkeys = [u \in Users |-> {}]
  /\ everHadPasskey = {}
  /\ issuedChallenge = {}
  /\ consumedChallenge = {}
  /\ issuedToken = {}
  /\ usedToken = {}
  /\ tokenOwner \in [Tokens -> Users]
  /\ activeSession = {}
  /\ endedSession = {}
  /\ registeredPasskey = [c \in Challenges |-> {}]
  /\ tokenOutcome = [t \in Tokens |-> {}]
  /\ loginSession = [c \in Challenges |-> {}]

IssueToken(t, u) ==
  /\ t \notin issuedToken
  /\ issuedToken' = issuedToken \cup {t}
  /\ tokenOwner' = [tokenOwner EXCEPT ![t] = u]
  /\ UNCHANGED <<passkeys, everHadPasskey, issuedChallenge, consumedChallenge,
                 usedToken, activeSession, endedSession, registeredPasskey,
                 tokenOutcome, loginSession>>

IssueChallenge(c) ==
  /\ c \notin issuedChallenge
  /\ issuedChallenge' = issuedChallenge \cup {c}
  /\ UNCHANGED <<passkeys, everHadPasskey, consumedChallenge, issuedToken,
                 usedToken, tokenOwner, activeSession, endedSession,
                 registeredPasskey, tokenOutcome, loginSession>>

Register(t, c, p) ==
  /\ t \in issuedToken \ usedToken
  /\ c \in issuedChallenge \ consumedChallenge
  /\ p \notin passkeys[tokenOwner[t]]
  /\ consumedChallenge' = consumedChallenge \cup {c}
  /\ usedToken' = usedToken \cup {t}
  /\ registeredPasskey' = [registeredPasskey EXCEPT ![c] = @ \cup {p}]
  /\ tokenOutcome' = [tokenOutcome EXCEPT ![t] = @ \cup {p}]
  /\ passkeys' = [passkeys EXCEPT ![tokenOwner[t]] = @ \cup {p}]
  /\ everHadPasskey' = everHadPasskey \cup {tokenOwner[t]}
  /\ UNCHANGED <<issuedChallenge, issuedToken, tokenOwner, activeSession,
                 endedSession, loginSession>>

Login(u, c, s) ==
  /\ passkeys[u] # {}
  /\ c \in issuedChallenge \ consumedChallenge
  /\ s \notin activeSession \cup endedSession
  /\ consumedChallenge' = consumedChallenge \cup {c}
  /\ loginSession' = [loginSession EXCEPT ![c] = @ \cup {s}]
  /\ activeSession' = activeSession \cup {s}
  /\ UNCHANGED <<passkeys, everHadPasskey, issuedChallenge, issuedToken,
                 usedToken, tokenOwner, endedSession, registeredPasskey,
                 tokenOutcome>>

AddPasskey(u, p) ==
  /\ p \notin passkeys[u]
  /\ passkeys' = [passkeys EXCEPT ![u] = @ \cup {p}]
  /\ everHadPasskey' = everHadPasskey \cup {u}
  /\ UNCHANGED <<issuedChallenge, consumedChallenge, issuedToken, usedToken,
                 tokenOwner, activeSession, endedSession, registeredPasskey,
                 tokenOutcome, loginSession>>

DeletePasskey(u, p) ==
  /\ p \in passkeys[u]
  (* 最後の 1 つは削除できない (FR-3)。 *)
  /\ Cardinality(passkeys[u]) > 1
  /\ passkeys' = [passkeys EXCEPT ![u] = @ \ {p}]
  /\ UNCHANGED <<everHadPasskey, issuedChallenge, consumedChallenge,
                 issuedToken, usedToken, tokenOwner, activeSession,
                 endedSession, registeredPasskey, tokenOutcome, loginSession>>

Logout(s) ==
  /\ s \in activeSession
  /\ activeSession' = activeSession \ {s}
  /\ endedSession' = endedSession \cup {s}
  /\ UNCHANGED <<passkeys, everHadPasskey, issuedChallenge, consumedChallenge,
                 issuedToken, usedToken, tokenOwner, registeredPasskey,
                 tokenOutcome, loginSession>>

ExpireChallenge(c) ==
  /\ c \in issuedChallenge \ consumedChallenge
  /\ issuedChallenge' = issuedChallenge \ {c}
  /\ UNCHANGED <<passkeys, everHadPasskey, consumedChallenge, issuedToken,
                 usedToken, tokenOwner, activeSession, endedSession,
                 registeredPasskey, tokenOutcome, loginSession>>

ExpireToken(t) ==
  /\ t \in issuedToken \ usedToken
  /\ issuedToken' = issuedToken \ {t}
  /\ UNCHANGED <<passkeys, everHadPasskey, issuedChallenge, consumedChallenge,
                 usedToken, tokenOwner, activeSession, endedSession,
                 registeredPasskey, tokenOutcome, loginSession>>

ExpireSession(s) ==
  /\ s \in activeSession
  /\ activeSession' = activeSession \ {s}
  /\ endedSession' = endedSession \cup {s}
  /\ UNCHANGED <<passkeys, everHadPasskey, issuedChallenge, consumedChallenge,
                 issuedToken, usedToken, tokenOwner, registeredPasskey,
                 tokenOutcome, loginSession>>

Next ==
  \/ \E t \in Tokens, u \in Users: IssueToken(t, u)
  \/ \E c \in Challenges: IssueChallenge(c)
  \/ \E t \in Tokens, c \in Challenges, p \in Passkeys: Register(t, c, p)
  \/ \E u \in Users, c \in Challenges, s \in Sessions: Login(u, c, s)
  \/ \E u \in Users, p \in Passkeys: AddPasskey(u, p)
  \/ \E u \in Users, p \in Passkeys: DeletePasskey(u, p)
  \/ \E s \in Sessions: Logout(s)
  \/ \E c \in Challenges: ExpireChallenge(c)
  \/ \E t \in Tokens: ExpireToken(t)
  \/ \E s \in Sessions: ExpireSession(s)

Spec == Init /\ [][Next]_vars

(* 検査する不変条件 (FR-1、FR-2、FR-3、FR-4)。 *)
Inv ==
  /\ TypeOK
  (* チャレンジは登録とログインを合わせて最大 1 回しか結果を生まない。 *)
  /\ \A c \in Challenges:
       Cardinality(registeredPasskey[c]) + Cardinality(loginSession[c]) <= 1
  (* 登録用トークンは最大 1 回しか使用されない。 *)
  /\ \A t \in Tokens: Cardinality(tokenOutcome[t]) <= 1
  (* パスキーを 1 つ以上持った利用者は、削除によって 0 個にならない。 *)
  /\ \A u \in Users: u \in everHadPasskey => passkeys[u] # {}
  (* ログアウトしたセッションは要求を通さない (再び有効にならない)。 *)
  /\ activeSession \cap endedSession = {}

=============================================================================
