//! Provider conversation ids are authoritative. RFC reference graphs fill the gaps.
use crate::Mail;
use std::collections::HashMap;

fn root(parent: &mut [usize], i: usize) -> usize {
    if parent[i] != i {
        parent[i] = root(parent, parent[i]);
    }
    parent[i]
}
pub fn threads(mails: &[Mail]) -> Vec<usize> {
    let mut parent: Vec<_> = (0..mails.len()).collect();
    let mut authority: Vec<_> = mails.iter().map(|m| m.remote_thread.clone()).collect();
    let mut remote = HashMap::new();
    let mut ids = HashMap::new();
    for (i, mail) in mails.iter().enumerate() {
        if !mail.message_id.is_empty() {
            ids.insert((mail.account.clone(), mail.message_id.clone()), i);
        }
        if !mail.remote_thread.is_empty() {
            if let Some(&other) = remote.get(&(mail.account.clone(), mail.remote_thread.clone())) {
                parent[i] = other;
            } else {
                remote.insert((mail.account.clone(), mail.remote_thread.clone()), i);
            }
        }
    }
    // Shared missing ancestors also connect replies when the original isn't downloaded.
    let mut ancestors = HashMap::new();
    for (i, mail) in mails.iter().enumerate() {
        for reference in &mail.references {
            let key = (mail.account.clone(), reference.clone());
            let other = ids
                .get(&key)
                .copied()
                .or_else(|| ancestors.get(&key).copied());
            ancestors.entry(key).or_insert(i);
            if let Some(other) = other {
                let a = root(&mut parent, i);
                let b = root(&mut parent, other);
                if a != b {
                    if authority[a].is_empty()
                        || authority[b].is_empty()
                        || authority[a] == authority[b]
                    {
                        parent[a] = b;
                        if authority[b].is_empty() {
                            authority[b] = authority[a].clone();
                        }
                    }
                }
            }
        }
    }
    (0..mails.len()).map(|i| root(&mut parent, i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_and_local_graphs_respect_account_and_provider_boundaries() {
        let mut m = crate::sample();
        m[0].message_id = "<a>".into();
        m[1].references = vec!["<a>".into()];
        m[2].references = vec!["<missing>".into()];
        m[3].references = vec!["<missing>".into()];
        m[4].remote_thread = "remote".into();
        m[5].remote_thread = "remote".into();
        m[6].remote_thread = "different".into();
        m[6].references = vec!["<a>".into()];
        m[0].remote_thread = "original".into();
        m[7].remote_thread = "remote".into();
        m[7].account = "other-account".into();
        let groups = threads(&m);
        assert_eq!(groups[0], groups[1]);
        assert_eq!(groups[2], groups[3]);
        assert_eq!(groups[4], groups[5]);
        assert_ne!(groups[0], groups[6]);
        assert_ne!(groups[4], groups[7]);
    }
}
