package com.pagi.adapter;

/**
 * Placeholder for enterprise-specific integrations.
 *
 * Intended design:
 * - talk to internal services (authz, catalogs, CRMs)
 * - translate enterprise request formats into canonical adapter responses
 */
public final class EnterpriseHandler {
    public String handle(String input) {
        return input;
    }
}

